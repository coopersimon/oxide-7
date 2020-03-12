// APU
// Consists of a CPU interface, the SPC-700 8-bit processor, and an 8-channel DSP.

mod dsp;
mod mem;
mod spc;
mod spcthread;

use spcthread::*;

use std::sync::{
    Arc,
    atomic::{
        AtomicU8,
        Ordering
    }
};

use crossbeam_channel::{
    unbounded,
    Sender
};

// CPU-side of APU. Sends and receives to/from audio thread, direct connection with CPU.
pub struct APU {
    command_tx:         Sender<SPCCommand>,

    ports_cpu_to_apu:   [Arc<AtomicU8>; 4],
    ports_apu_to_cpu:   [Arc<AtomicU8>; 4],

    spc_thread:         SPCThread
}

impl APU {
    pub fn new() -> Self {
        let (command_tx, command_rx) = unbounded();

        let ports_cpu_to_apu = [Arc::new(AtomicU8::new(0)), Arc::new(AtomicU8::new(0)), Arc::new(AtomicU8::new(0)), Arc::new(AtomicU8::new(0))];
        let ports_apu_to_cpu = [Arc::new(AtomicU8::new(0)), Arc::new(AtomicU8::new(0)), Arc::new(AtomicU8::new(0)), Arc::new(AtomicU8::new(0))];

        APU {
            command_tx:         command_tx,

            ports_cpu_to_apu:   [ports_cpu_to_apu[0].clone(), ports_cpu_to_apu[1].clone(), ports_cpu_to_apu[2].clone(), ports_cpu_to_apu[3].clone()],
            ports_apu_to_cpu:   [ports_apu_to_cpu[0].clone(), ports_apu_to_cpu[1].clone(), ports_apu_to_cpu[2].clone(), ports_apu_to_cpu[3].clone()],

            spc_thread:         SPCThread::new(command_rx, ports_cpu_to_apu, ports_apu_to_cpu)
        }
    }

    pub fn clock(&mut self, cycles: usize) {
        self.command_tx.send(SPCCommand::Clock(cycles)).unwrap();
    }

    pub fn read_port_0(&mut self) -> u8 {
        let data = self.ports_apu_to_cpu[0].load(Ordering::SeqCst);
        //println!("CPU Read {:X} from port 0", data);
        data
    }

    pub fn read_port_1(&mut self) -> u8 {
        let data = self.ports_apu_to_cpu[1].load(Ordering::SeqCst);
        //println!("CPU Read {:X} from port 1", data);
        data
    }

    pub fn read_port_2(&mut self) -> u8 {
        let data = self.ports_apu_to_cpu[2].load(Ordering::SeqCst);
        //println!("CPU Read {:X} from port 2", data);
        data
    }

    pub fn read_port_3(&mut self) -> u8 {
        let data = self.ports_apu_to_cpu[3].load(Ordering::SeqCst);
        //println!("CPU Read {:X} from port 3", data);
        data
    }

    pub fn write_port_0(&mut self, data: u8) {
        self.ports_cpu_to_apu[0].store(data, Ordering::SeqCst);
        //println!("CPU Write {:X} to port 0", data);
    }

    pub fn write_port_1(&mut self, data: u8) {
        self.ports_cpu_to_apu[1].store(data, Ordering::SeqCst);
        //println!("CPU Write {:X} to port 1", data);
    }

    pub fn write_port_2(&mut self, data: u8) {
        self.ports_cpu_to_apu[2].store(data, Ordering::SeqCst);
        //println!("CPU Write {:X} to port 2", data);
    }

    pub fn write_port_3(&mut self, data: u8) {
        self.ports_cpu_to_apu[3].store(data, Ordering::SeqCst);
        //println!("CPU Write {:X} to port 3", data);
    }
}