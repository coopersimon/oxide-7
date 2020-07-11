// APU
// Consists of a CPU interface, the SPC-700 8-bit processor, and an 8-channel DSP.

mod dsp;
mod mem;
mod resampler;
mod spc;

use crossbeam_channel::{
    unbounded,
    Receiver
};

use sample::frame::Stereo;

use crate::constants;

use spc::SPC;
use mem::SPCBus;

pub type SamplePacket = Box<[Stereo<f32>]>;
pub use resampler::Resampler;
    
const SPC_CLOCK_RATE: usize = 1_024_000;
const SPC_RATIO: f64 = (SPC_CLOCK_RATE as f64) / (constants::timing::MASTER_HZ as f64); // Around 1/21

// The APU processes SPC instructions and generates audio.
pub struct APU {
    signal_rx:      Option<Receiver<SamplePacket>>, // Receiver that will be used on the audio thread.

    spc:            SPC<SPCBus>,
    cycle_count:    f64,
}

impl APU {
    pub fn new() -> Self {
        let (signal_tx, signal_rx) = unbounded();
        let bus = SPCBus::new(signal_tx);

        APU {
            signal_rx:      Some(signal_rx),

            spc:            SPC::new(bus),
            cycle_count:    0.0
        }
    }

    pub fn get_rx(&mut self) -> Option<Receiver<SamplePacket>> {
        std::mem::replace(&mut self.signal_rx, None)
    }

    pub fn clock(&mut self, cycles: usize) {
        self.cycle_count += calc_cycles(cycles);

        while self.cycle_count > 0.0 {
            let cycles_passed = self.spc.step() as f64;
            self.cycle_count -= cycles_passed;
        }
    }

    pub fn read_port(&self, port_num: usize) -> u8 {
        self.spc.read_port(port_num)
    }

    pub fn write_port(&mut self, port_num: usize, data: u8) {
        self.spc.write_port(port_num, data);
    }
}

// Convert master cycles into SPC cycles.
// SNES clock: 21_442_080 Hz
// SPC clock: 1_024_000 Hz
fn calc_cycles(master_cycles: usize) -> f64 {
    (master_cycles as f64) * SPC_RATIO
}