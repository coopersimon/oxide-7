// Memory for the SPC-700.
mod timer;

use bitflags::bitflags;
use crossbeam_channel::Sender;

use crate::mem::RAM;
use timer::Timer;
use super::dsp::DSP;

bitflags! {
    struct SPCControl: u8 {
        const ROM_ENABLE =      bit!(7);
        const CLEAR_PORT_32 =   bit!(5);
        const CLEAR_PORT_10 =   bit!(4);
        const ENABLE_TIMER_2 =  bit!(2);
        const ENABLE_TIMER_1 =  bit!(1);
        const ENABLE_TIMER_0 =  bit!(0);
    }
}

const SPC_RAM_SIZE: usize = 1024 * 64;  // 64KB of RAM.

const IPL_ROM: [u8; 64] = [
   0xCD, 0xEF, 0xBD, 0xE8, 0x00, 0xC6, 0x1D, 0xD0,
   0xFC, 0x8F, 0xAA, 0xF4, 0x8F, 0xBB, 0xF5, 0x78,
   0xCC, 0xF4, 0xD0, 0xFB, 0x2F, 0x19, 0xEB, 0xF4,
   0xD0, 0xFC, 0x7E, 0xF4, 0xD0, 0x0B, 0xE4, 0xF5,
   0xCB, 0xF4, 0xD7, 0x00, 0xFC, 0xD0, 0xF3, 0xAB,
   0x01, 0x10, 0xEF, 0x7E, 0xF4, 0x10, 0xEB, 0xBA,
   0xF6, 0xDA, 0x00, 0xBA, 0xF4, 0xC4, 0xF4, 0xDD,
   0x5D, 0xD0, 0xDB, 0x1F, 0x00, 0x00, 0xC0, 0xFF
];

pub trait SPCMem {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
    fn clock(&mut self, cycles: usize);
}

pub struct SPCBus {
    ram:                RAM,

    ipl_rom:            &'static [u8; 64],

    // Registers
    control:            SPCControl,
    dsp_reg_addr:       u8,
    dsp:                DSP,

    // Port data sent in from CPU.
    ports_cpu_to_apu:   [u8; 4],

    // Port data sent out from APU.
    ports_apu_to_cpu:   [u8; 4],

    timer_0:            Timer,
    timer_1:            Timer,
    timer_2:            Timer,
}

impl SPCBus {
    pub fn new(signal_tx: Sender<super::SamplePacket>) -> Self {
        SPCBus {
            ram:        RAM::new(SPC_RAM_SIZE),

            ipl_rom:    &IPL_ROM,

            control:        SPCControl::ROM_ENABLE | SPCControl::CLEAR_PORT_32 | SPCControl::CLEAR_PORT_10,
            dsp_reg_addr:   0,
            dsp:            DSP::new(signal_tx),

            ports_cpu_to_apu:   [0; 4],
            ports_apu_to_cpu:   [0; 4],

            timer_0:        Timer::new(128),
            timer_1:        Timer::new(128),
            timer_2:        Timer::new(16),
        }
    }

    pub fn read_port(&self, port_num: usize) -> u8 {
        self.ports_apu_to_cpu[port_num]
    }

    pub fn write_port(&mut self, port_num: usize, data: u8) {
        self.ports_cpu_to_apu[port_num] = data;
    }
}

impl SPCMem for SPCBus {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0xF1 => 0,

            0xF2 => self.dsp_reg_addr,
            0xF3 => self.dsp.read(self.dsp_reg_addr & 0x7F),

            0xF4 => self.ports_cpu_to_apu[0],
            0xF5 => self.ports_cpu_to_apu[1],
            0xF6 => self.ports_cpu_to_apu[2],
            0xF7 => self.ports_cpu_to_apu[3],

            0xFA..=0xFC => 0,

            0xFD => self.timer_0.read_counter(),
            0xFE => self.timer_1.read_counter(),
            0xFF => self.timer_2.read_counter(),

            0xFFC0..=0xFFFF if self.control.contains(SPCControl::ROM_ENABLE) => self.ipl_rom[(addr - 0xFFC0) as usize],

            _ => self.ram.read(addr.into())
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0xF1 => self.set_control(data),

            0xF2 => self.dsp_reg_addr = data,
            0xF3 => self.dsp.write(self.dsp_reg_addr & 0x7F, data, &self.ram),

            0xF4 => self.ports_apu_to_cpu[0] = data,
            0xF5 => self.ports_apu_to_cpu[1] = data,
            0xF6 => self.ports_apu_to_cpu[2] = data,
            0xF7 => self.ports_apu_to_cpu[3] = data,

            0xFA => self.timer_0.write_timer_modulo(data),
            0xFB => self.timer_1.write_timer_modulo(data),
            0xFC => self.timer_2.write_timer_modulo(data),

            0xFD..=0xFF => {},

            _ => self.ram.write(addr.into(), data)
        }
    }

    fn clock(&mut self, cycles: usize) {
        if self.control.contains(SPCControl::ENABLE_TIMER_0) {
            self.timer_0.clock(cycles);
        }
        if self.control.contains(SPCControl::ENABLE_TIMER_1) {
            self.timer_1.clock(cycles);
        }
        if self.control.contains(SPCControl::ENABLE_TIMER_2) {
            self.timer_2.clock(cycles);
        }

        self.dsp.clock(cycles, &mut self.ram);
    }
}

impl SPCBus {
    fn set_control(&mut self, data: u8) {
        let control = SPCControl::from_bits_truncate(data);

        self.timer_0.reset();
        self.timer_1.reset();
        self.timer_2.reset();

        if control.contains(SPCControl::CLEAR_PORT_10) {
            //println!("APU reset ports 0 and 1");
            self.ports_cpu_to_apu[0] = 0;
            self.ports_cpu_to_apu[1] = 0;
        }
        if control.contains(SPCControl::CLEAR_PORT_32) {
            //println!("APU reset ports 2 and 3");
            self.ports_cpu_to_apu[2] = 0;
            self.ports_cpu_to_apu[3] = 0;
        }

        self.control = control;
    }
}