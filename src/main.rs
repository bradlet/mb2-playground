//! main.rs

#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use lsm303agr::{AccelMode, AccelOutputDataRate, Lsm303agr};
use cortex_m_rt::entry;
use microbit::{
	board::Board,
	display::nonblocking::{Display, GreyscaleImage},
	hal::{
		pac::{self, TIMER1, interrupt},
		prelude::*,
		Timer,
		gpio::Level,
		twim,
	},
	pac::twim0::frequency::FREQUENCY_A
};

use critical_section_lock_mut::LockMut;

/// The display is shared by the main program and the
/// interrupt handler.
static DISPLAY: LockMut<Display<TIMER1>> = LockMut::new();

const BASE_FREQ: u16 = 100u16;
const OFFSET_TURN_AT: u16 = 100u16;

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let mut board = Board::take().unwrap();
	let mut timer = Timer::new(board.TIMER0);

	let display = Display::new(board.TIMER1, board.display_pins);
	DISPLAY.init(display);

    let mut speaker = board.speaker_pin.into_push_pull_output(Level::Low);

	// https://docs.rust-embedded.org/discovery/microbit/08-i2c/using-a-driver.html
	let i2c = { twim::Twim::new(board.TWIM0, board.i2c_internal.into(), FREQUENCY_A::K100) };
	let mut sensor = Lsm303agr::new_with_i2c(i2c);
	sensor.init().unwrap();
	sensor.set_accel_mode_and_odr(&mut timer, AccelMode::Normal, AccelOutputDataRate::Hz50).unwrap();

    let button = board.buttons.button_a;

	let mut increasing = true;
	let mut off_set = 0u16;

	unsafe {
        board.NVIC.set_priority(pac::Interrupt::TIMER1, 128);
        pac::NVIC::unmask(pac::Interrupt::TIMER1);
    }
	
	rprintln!("Starting...");

	let image = GreyscaleImage::new(&[
        [0, 1, 9, 1, 0],
        [0, 1, 9, 1, 0],
        [0, 1, 9, 1, 0],
        [0, 1, 1, 1, 0],
        [0, 0, 9, 0, 0],
    ]);

	loop {
		// https://crates.io/crates/lsm303agr/0.3.0
		if sensor.accel_status().unwrap().xyz_new_data() {
            let data = sensor.acceleration().unwrap();
			let linear_acceleration_sq = data.x_mg() * data.x_mg() + data.y_mg() * data.y_mg() + data.z_mg() * data.z_mg();
            rprintln!("Linear acceleration: {}", linear_acceleration_sq);
        }
		if button.is_low().unwrap() {
			DISPLAY.with_lock(|display| display.show(&image));
			speaker.set_high().unwrap();
			rprintln!("HIGH");
			timer.delay_us(BASE_FREQ + off_set);
            speaker.set_low().unwrap();
			rprintln!("LOW");
            timer.delay_us(BASE_FREQ + off_set);
			// Add offset so my scream can waver.
			if increasing {
				off_set += 1;
			} else {
				off_set -= 1;
			}
			if off_set == 0 && !increasing {
				increasing = true;
			} else if off_set == OFFSET_TURN_AT && increasing {
				increasing = false;
			}
		};
		DISPLAY.with_lock(|display| display.clear());
	}
}

#[interrupt]
fn TIMER1() {
    DISPLAY.with_lock(|display| display.handle_display_event());
}