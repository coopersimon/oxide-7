// Deals with communication between APU on CPU side, and SPC thread.

use std::sync::{
    Arc,
    atomic::AtomicU8
};

use crossbeam_channel::Receiver;

use std::thread;

use super::{
    spc::SPC,
    mem::SPCBus
};
use crate::constants;

const SPC_CLOCK_RATE: usize = 1024000;

// Commands that can be sent to the SPC thread.
pub enum SPCCommand {
    Clock(usize)    // Sends how many SNES master cycles have passed.
}

pub struct SPCThread {
    thread: thread::JoinHandle<()>
}

impl SPCThread {
    pub fn new(rx: Receiver<SPCCommand>, ports_cpu_to_apu: [Arc<AtomicU8>; 4], ports_apu_to_cpu: [Arc<AtomicU8>; 4]) -> Self {
        let thread = thread::spawn(move || {
            let bus = SPCBus::new(ports_cpu_to_apu, ports_apu_to_cpu);
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