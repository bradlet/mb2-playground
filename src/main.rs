//! main.rs

#![no_std]
#![no_main]

use core::cell::RefCell;

use cortex_m::interrupt::Mutex;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use lsm303agr::{AccelMode, AccelOutputDataRate, Lsm303agr};
use cortex_m_rt::entry;
use microbit::{
	board::Board, display::nonblocking::Display, hal::{
		gpio::Level, pac::{self, interrupt, TIMER1}, prelude::*, pwm, rtc::RtcInterrupt, time::Hertz, twim, Clocks, Rtc, Timer
	}, pac::twim0::frequency::FREQUENCY_A
};

use critical_section_lock_mut::LockMut;

use mb2_playground::*;

/// The display is shared by the main program and the
/// interrupt handler.
static DISPLAY: LockMut<Display<TIMER1>> = LockMut::new();
static RTC: Mutex<RefCell<Option<Rtc<pac::RTC0>>>> = Mutex::new(RefCell::new(None));
static SPEAKER: Mutex<RefCell<Option<pwm::Pwm<pac::PWM0>>>> = Mutex::new(RefCell::new(None));

static mut FALLING: bool = false;

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let mut board = Board::take().unwrap();
	let mut timer = Timer::new(board.TIMER0);

	let display = Display::new(board.TIMER1, board.display_pins);
	DISPLAY.init(display);

	// https://docs.rust-embedded.org/discovery/microbit/08-i2c/using-a-driver.html
	let i2c = { twim::Twim::new(board.TWIM0, board.i2c_internal.into(), FREQUENCY_A::K100) };
	let mut sensor = Lsm303agr::new_with_i2c(i2c);
	sensor.init().unwrap();
	sensor.set_accel_mode_and_odr(&mut timer, AccelMode::Normal, AccelOutputDataRate::Hz50).unwrap();

    let button = board.buttons.button_a; // TODO remove

	unsafe {
        board.NVIC.set_priority(pac::Interrupt::TIMER1, 128);
        pac::NVIC::unmask(pac::Interrupt::TIMER1);
    }

	let mut machine: StateMachine<FallingState> = StateMachine::new();
	
	rprintln!("Starting...");

	let image = get_default_grayscale_image();

	// Copied here https://github.com/pdx-cs-rust-embedded/mb2-audio-experiments/blob/v2-speaker-demo/src/main.rs
	cortex_m::interrupt::free(move |cs| {
        // NB: The LF CLK pin is used by the speaker
        let _clocks = Clocks::new(board.CLOCK)
            .enable_ext_hfosc()
            .set_lfclk_src_synth()
            .start_lfclk();

        // Set up ticks: TICK = 32768 / (d + 1), so d = 32768 / TICK - 1.
        let mut rtc = Rtc::new(board.RTC0, 32768 / TICK - 1).unwrap();
        rtc.enable_counter();
        rtc.enable_interrupt(RtcInterrupt::Tick, Some(&mut board.NVIC));
        rtc.enable_event(RtcInterrupt::Tick);

        *RTC.borrow(cs).borrow_mut() = Some(rtc);

        let mut speaker_pin = board.speaker_pin.into_push_pull_output(Level::High);
        speaker_pin.set_low().unwrap();

        // Use the PWM peripheral to generate a waveform for the speaker
        let speaker = pwm::Pwm::new(board.PWM0);
        speaker
            // output the waveform on the speaker pin
            .set_output_pin(pwm::Channel::C0, speaker_pin.degrade())
            .set_prescaler(pwm::Prescaler::Div4)
            // Initial frequency
			.set_period(Hertz(BASE_FREQ))
            .set_seq_refresh(pwm::Seq::Seq0, 0)
            .set_seq_end_delay(pwm::Seq::Seq0, 0)
            .set_max_duty(32767)
			.disable();

        *SPEAKER.borrow(cs).borrow_mut() = Some(speaker);

        // Configure RTC interrupt
        unsafe {
            pac::NVIC::unmask(pac::Interrupt::RTC0);
        }
        pac::NVIC::unpend(pac::Interrupt::RTC0);
    });
	// Copy end here

	loop {
		// https://crates.io/crates/lsm303agr/0.3.0
		// let transition = if sensor.accel_status().unwrap().xyz_new_data() {
        //     let data = sensor.acceleration().unwrap();
		// 	// Acceleration in milli-g
		// 	rprintln!("Acceleration [x, y, z]: [{}, {}, {}]", data.x_mg(), data.y_mg(), data.z_mg());
		// 	// let linear_acceleration_sq = data.x_mg() * data.x_mg() + data.y_mg() * data.y_mg() + data.z_mg() * data.z_mg();
        //     // rprintln!("Linear acceleration: {}", linear_acceleration_sq);
		// 	FallingStateInput::Fall
        // } else {
		// 	FallingStateInput::Stop
		// };

		let transition = if button.is_low().unwrap() {
			FallingStateInput::Fall
		} else {
			FallingStateInput::Stop
		};

		match machine.consume(&transition) {
			Ok(Some(reaction)) => {
				match reaction {
					FallingStateOutput::Fall => {
						DISPLAY.with_lock(|display| display.show(&image));
						// make_sin_wave(&mut speaker, &mut timer);
						unsafe {
							FALLING = true;
						}
					}
					FallingStateOutput::Stop => {
						DISPLAY.with_lock(|display| display.clear());
						unsafe {
							FALLING = false;
						}
					}
				}
			}
			_ => { /* Do nothing */ }
		}
	}
}

#[interrupt]
fn TIMER1() {
    DISPLAY.with_lock(|display| display.handle_display_event());
}

// RTC interrupt, exectued for each RTC tick
// https://github.com/pdx-cs-rust-embedded/mb2-audio-experiments/blob/v2-speaker-demo/src/main.rs
#[interrupt]
fn RTC0() {
    /* Enter critical section */
    cortex_m::interrupt::free(|cs| {
        /* Borrow devices */
        if let (Some(speaker), Some(rtc)) = (
            SPEAKER.borrow(cs).borrow().as_ref(),
            RTC.borrow(cs).borrow().as_ref(),
        ) {
			unsafe {
				if FALLING {
					rprintln!("FALLING");
					speaker.set_period(Hertz(BASE_FREQ));
					speaker.enable();
					// Restart the PWM at duty cycle
					let max_duty = speaker.max_duty() as u32;
					let duty = DUTY * max_duty / 100;
					let duty = duty.clamp(1, max_duty / 2);
					speaker.set_duty_on_common(duty as u16);
				} else {
					rprintln!("STILL");
					speaker.disable();
				}
			}

            // Clear the RTC interrupt
            rtc.reset_event(RtcInterrupt::Tick);
        }
    });
}