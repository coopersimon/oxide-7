// Deals with communication between APU on CPU side, and SPC thread.

use std::sync::{
    Arc,
    Mutex
};

use crossbeam_channel::Receiver;

use std::thread;

use super::spc::{
    SPC,
    SPCBus
};
use crate::constants;

const SPC_CLOCK_RATE: usize = 1024000;

// Commands that can be sent to the SPC thread.
pub enum SPCCommand {
    // Write a value to the SPC-700.
    Port0Write(u8),
    Port1Write(u8),
    Port2Write(u8),
    Port3Write(u8),

    Clock(usize)    // Sends how many SNES master cycles have passed.
}

pub struct SPCThread {
    thread: thread::JoinHandle<()>
}

impl SPCThread {
    pub fn new(rx: Receiver<SPCCommand>, ports: [Arc<Mutex<u8>>; 4]) -> Self {
        let thread = thread::spawn(move || {
            let bus = SPCBus::new(ports);
            let mut spc = SPC::new(bus);
            let mut cycle_count = 0.0;

            while let Ok(command) = rx.recv() {
                match command {
                    SPCCommand::Clock(c) => {
                        cycle_count += calc_cycles(c);

                        while cycle_count > 0.0 {
                            let cycles_passed = spc.step() as f64;
                            cycle_count -= cycles_passed;
                        }
                    },

                    SPCCommand::Port0Write(d) => spc.write_port(0, d),
                    SPCCommand::Port1Write(d) => spc.write_port(1, d),
                    SPCCommand::Port2Write(d) => spc.write_port(2, d),
                    SPCCommand::Port3Write(d) => spc.write_port(3, d),
                }
            }
        });

        SPCThread {
            thread: thread
        }
    }
}

// Convert master cycles into SPC cycles.
// SNES clock: 21_442_080 Hz
// SPC clock: 1_024_000 Hz
fn calc_cycles(master_cycles: usize) -> f64 {
    let frac = (master_cycles as f64) / (constants::timing::MASTER_HZ as f64);

    frac * (SPC_CLOCK_RATE as f64)
}