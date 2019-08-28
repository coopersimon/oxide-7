#[macro_use]
mod common;
mod cpu;
mod mem;

fn main() {
    let mut cpu = cpu::CPU::new();

    loop {
        cpu.step();
    }
}
