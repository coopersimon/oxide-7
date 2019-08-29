#[macro_use]
mod common;
mod constants;

mod cpu;
mod joypad;
mod mem;

mod snes;

mod debug;

fn main() {
    let cart_path = std::env::args().nth(1).expect("Expected ROM file path as first argument!");

    let debug_mode = std::env::args().nth(2).is_some();

    let mut snes = snes::SNES::new(&cart_path, "");

    if debug_mode {
        debug::debug_mode(&mut snes.cpu);
    } else {
        loop {
            snes.step();
        }
    }

}
