#![no_std]

use microbit::display::nonblocking::GreyscaleImage;

pub use rust_fsm::*;

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
