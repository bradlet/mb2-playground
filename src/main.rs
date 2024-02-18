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

	unsafe {
        board.NVIC.set_priority(pac::Interrupt::TIMER1, 128);
        pac::NVIC::unmask(pac::Interrupt::TIMER1);
    }
	
	rprintln!("Starting...");

	let image = GreyscaleImage::new(&[
        [0, 4, 0, 4, 0],
        [7, 3, 7, 3, 7],
        [7, 2, 2, 2, 7],
        [0, 7, 2, 7, 0],
        [0, 0, 7, 0, 0],
    ]);

	loop {
		if button.is_low().unwrap() {
			speaker.set_high().unwrap();
			rprintln!("HIGH");
			delay.delay_us(3_000u16);
            speaker.set_low().unwrap();
			rprintln!("LOW");
            delay.delay_us(3_000u16);
		};
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