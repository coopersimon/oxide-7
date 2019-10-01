// APU
// Consists of a CPU interface, the SPC-700 8-bit processor, and an 8-channel DSP.

mod spc;
mod spcthread;

use spcthread::*;

use std::sync::mpsc::{
    channel,
    Sender,
    Receiver
};

// CPU-side of APU. Sends and receives to/from audio thread, direct connection with CPU.
pub struct APU {
    command_tx:     Sender<SPCCommand>,
    port_data_rx:   Receiver<SPCPortData>,

    port_0:         u8,
    port_1:         u8,
    port_2:         u8,
    port_3:         u8,

    spc_thread:     SPCThread
}

impl APU {
    pub fn new() -> Self {
        let (command_tx, command_rx) = channel();
        let (port_data_tx, port_data_rx) = channel();

        APU {
            command_tx:     command_tx,
            port_data_rx:   port_data_rx,

            port_0:         0,
            port_1:         0,
            port_2:         0,
            port_3:         0,

            spc_thread:     SPCThread::new(command_rx, port_data_tx)
        }
    }

    pub fn clock(&mut self, cycles: usize) {
        self.command_tx.send(SPCCommand::Clock(cycles)).unwrap();
    }

    pub fn read_port_0(&mut self) -> u8 {
        self.refresh_ports();
        self.port_0
    }

    pub fn read_port_1(&mut self) -> u8 {
        self.refresh_ports();
        self.port_1
    }

    pub fn read_port_2(&mut self) -> u8 {
        self.refresh_ports();
        self.port_2
    }

    pub fn read_port_3(&mut self) -> u8 {
        self.refresh_ports();
        self.port_3
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

impl APU {
    // Check port data to see if there is anything new and store the latest data.
    fn refresh_ports(&mut self) {
        use SPCPortData::*;

        for d in self.port_data_rx.try_iter() {
            match d {
                Port0(d) => self.port_0 = d,
                Port1(d) => self.port_1 = d,
                Port2(d) => self.port_2 = d,
                Port3(d) => self.port_3 = d,
            }
        }
    }
}