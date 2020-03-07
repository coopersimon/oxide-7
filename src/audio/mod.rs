// APU
// Consists of a CPU interface, the SPC-700 8-bit processor, and an 8-channel DSP.

mod spc;
mod spcthread;

use spcthread::*;

use std::sync::{
    Arc,
    Mutex
};

use crossbeam_channel::{
    unbounded,
    Sender
};

// CPU-side of APU. Sends and receives to/from audio thread, direct connection with CPU.
pub struct APU {
    command_tx:     Sender<SPCCommand>,

    port_0:         Arc<Mutex<u8>>,
    port_1:         Arc<Mutex<u8>>,
    port_2:         Arc<Mutex<u8>>,
    port_3:         Arc<Mutex<u8>>,

    spc_thread:     SPCThread
}

impl APU {
    pub fn new() -> Self {
        let (command_tx, command_rx) = unbounded();

        let ports = [Arc::new(Mutex::new(0)), Arc::new(Mutex::new(0)), Arc::new(Mutex::new(0)), Arc::new(Mutex::new(0))];

        APU {
            command_tx:     command_tx,

            port_0:         ports[0].clone(),
            port_1:         ports[1].clone(),
            port_2:         ports[2].clone(),
            port_3:         ports[3].clone(),

            spc_thread:     SPCThread::new(command_rx, ports)
        }
    }

    pub fn clock(&mut self, cycles: usize) {
        self.command_tx.send(SPCCommand::Clock(cycles)).unwrap();
    }

    pub fn read_port_0(&mut self) -> u8 {
        *self.port_0.lock().unwrap()
    }

    pub fn read_port_1(&mut self) -> u8 {
        *self.port_1.lock().unwrap()
    }

    pub fn read_port_2(&mut self) -> u8 {
        *self.port_2.lock().unwrap()
    }

    pub fn read_port_3(&mut self) -> u8 {
        *self.port_3.lock().unwrap()
    }

    pub fn write_port_0(&mut self, data: u8) {
        self.command_tx.send(SPCCommand::Port0Write(data)).unwrap();
    }

    pub fn write_port_1(&mut self, data: u8) {
        self.command_tx.send(SPCCommand::Port1Write(data)).unwrap();
    }

    pub fn write_port_2(&mut self, data: u8) {
        self.command_tx.send(SPCCommand::Port2Write(data)).unwrap();
    }

    pub fn write_port_3(&mut self, data: u8) {
        self.command_tx.send(SPCCommand::Port3Write(data)).unwrap();
    }
}