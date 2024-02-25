#![no_std]

use microbit::{
	display::nonblocking::GreyscaleImage,
	hal::{
		gpio::{
			p0::P0_00,
			Output,
			PushPull,
		},
		Timer,
		prelude::*,
	},
	pac::TIMER0,
};
use rtt_target::rprintln;

pub use rust_fsm::*;

pub const BASE_FREQ: u16 = 50u16;

state_machine! {
    derive(Debug)
    repr_c(true)
    FallingState(Still)

    Still(Fall) => Falling,
    Falling(Stop) => Recovering,
    Recovering => {
        Stop => Still,
        Fall => Falling,
    }
}

pub fn get_default_grayscale_image() -> GreyscaleImage {
	GreyscaleImage::new(&[
		[0, 1, 9, 1, 0],
		[0, 1, 9, 1, 0],
		[0, 1, 9, 1, 0],
		[0, 1, 1, 1, 0],
		[0, 0, 9, 0, 0],
	])
}

pub fn make_sin_wave(speaker: &mut P0_00<Output<PushPull>>, timer: &mut Timer<TIMER0>) {
	speaker.set_high().unwrap();
	rprintln!("HIGH");
	timer.delay_us(BASE_FREQ);
	speaker.set_low().unwrap();
	rprintln!("LOW");
	timer.delay_us(BASE_FREQ);
}