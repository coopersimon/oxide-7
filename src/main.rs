#[macro_use]
mod common;
mod cpu;
mod mem;

fn main() {

    let mut bus = mem::MemBus::new("");
    let mut cpu = cpu::CPU::new(bus);

    loop {
        cpu.step();
    }
}
