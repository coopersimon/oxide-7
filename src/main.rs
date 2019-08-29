#[macro_use]
mod common;
mod cpu;
mod joypad;
mod mem;
mod timing;

mod debug;

fn main() {
    let cart_name = std::env::args().nth(1).expect("Expected ROM file path as first argument!");

    let bus = mem::MemBus::new(&cart_name);
    let mut cpu = cpu::CPU::new(bus);

    let debug_mode = std::env::args().nth(2).is_some();

    if debug_mode {
        debug::debug_mode(&mut cpu);
    } else {
        loop {
            cpu.step();
        }
    }

}
