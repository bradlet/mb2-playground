#![no_std]

use microbit::display::nonblocking::GreyscaleImage;

pub use rust_fsm::*;

pub const TICK: u32 = 16;
pub const BASE_FREQ: u32 = 700;
pub const DUTY: u32 = 1000;

state_machine! {
    derive(Debug)
    repr_c(true)
    pub FallingState(Still)

    Still(Fall) => Falling [Fall],
	Falling(Stop) => Still [Stop],
}

pub fn get_default_grayscale_image() -> GreyscaleImage {
	GreyscaleImage::new(&[
		[0, 1, 9, 1, 0],
		[0, 1, 9, 1, 0],
		[0, 1, 9, 1, 0],
		[0, 1, 1, 1, 0],
		[0, 1, 9, 1, 0],
	])
}
