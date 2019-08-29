#[macro_use]
mod common;
mod cpu;
mod mem;

fn main() {
    let cart_name = std::env::args().nth(1).expect("Expected ROM file path as first argument!");

    let bus = mem::MemBus::new(&cart_name);
    let mut cpu = cpu::CPU::new(bus);

    loop {
        cpu.step();
    }
}
