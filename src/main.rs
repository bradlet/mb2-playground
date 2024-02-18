//! main.rs

#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use cortex_m_rt::entry;
use microbit::{
	board::Board,
	display::nonblocking::{Display, GreyscaleImage},
	hal::{
		pac::{self, TIMER1, interrupt},
		prelude::*,
		Timer,
		delay::Delay,
		gpio::Level,
	},
};

use critical_section_lock_mut::LockMut;

/// The display is shared by the main program and the
/// interrupt handler.
static DISPLAY: LockMut<Display<TIMER1>> = LockMut::new();

const BASE_FREQ: u16 = 100u16;
const OFFSET_TURN_AT: u16 = 250u16;

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let mut board = Board::take().unwrap();
	let mut timer = Timer::new(board.TIMER0);
	let display = Display::new(board.TIMER1, board.display_pins);
	DISPLAY.init(display);

	let mut delay = Delay::new(board.SYST);
    let mut speaker = board.speaker_pin.into_push_pull_output(Level::Low);
    let button = board.buttons.button_a;

	let mut count = 0u16;

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

			// DISPLAY.with_lock(|display| display.show(&image));
			// timer.delay_ms(2000u32);
		};
		DISPLAY.with_lock(|display| display.clear());
		// count += 1;
		// rprintln!("After A process {}", count);
		// DISPLAY.with_lock(|display| display.clear());
		// DISPLAY.with_lock(|display| display.show(&image));
		// timer.delay_ms(2000u32);

		// DISPLAY.with_lock(|display| display.clear());
		// timer.delay_ms(2000u32);
	}
}

#[interrupt]
fn TIMER1() {
    DISPLAY.with_lock(|display| display.handle_display_event());
}