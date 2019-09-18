// Address Buses A and B, and DMA operation.

use std::{
    io::{
        BufReader,
        Read,
        Seek,
        SeekFrom
    },
    fs::File
};

use crate::{
    common::Interrupt,
    constants::timing::*,
    video::{PPU, PPUSignal},
    audio::APU
};

use super::{
    RAM,
    dma::{
        DMAChannel,
        DMAControl
    },
    rom::{
        Cart,
        LoROM,
        HiROM
    }
};

// A bus to attach to the CPU (Address Bus A).
pub struct MemBus {
    // Devices
    bus_b:      AddrBusB,

    // Memory
    cart:       Box<dyn Cart>,
    wram:       RAM,

    // Stored values
    wram_addr:      u32,
    mult_operand:   u8,
    div_operand:    u16,
    div_result:     u16,    // TODO: do these need to be timed properly?
    mult_result:    u16,

    // DMA
    hdma_active:    u8,
    dma_channels:   Vec<DMAChannel>,

    // Extensions
}

impl MemBus {
    pub fn new(cart_path: &str) -> Self {
        // Open ROM file.
        let f = File::open(cart_path).expect(&format!("Couldn't open file {}", cart_path));

        let reader = BufReader::new(f);
        
        let cart = MemBus::make_cart(reader);

        MemBus {
            bus_b:      AddrBusB::new(),
            //joypads:    JoypadMem::new(),
            
            cart:       cart,
            wram:       RAM::new(0x20000),

            hdma_active:    0,
            dma_channels:   vec![DMAChannel::new(); 8],

            wram_addr:      0,
            mult_operand:   0,
            div_operand:    0,
            div_result:     0,
            mult_result:    0,
        }
    }

    pub fn read(&mut self, addr: u32) -> (u8, usize) {
        let bank = hi24!(addr);
        let offset = lo24!(addr);

        match bank {
            0x00..=0x3F | 0x80..=0xBF => match offset {
                0x0000..=0x1FFF => (self.wram.read(offset as u32), SLOW_MEM_ACCESS),

                0x2134..=0x2143 => (self.bus_b.read(lo!(offset)), FAST_MEM_ACCESS),
                0x2180          => self.read(self.wram_addr),
                0x2100..=0x21FF => (0, FAST_MEM_ACCESS),
                0x3000..=0x3FFF => (0, FAST_MEM_ACCESS),                                // Extensions

                0x4000..=0x4015 |
                0x4000..=0x41FF => (self.bus_b.ppu.read_joypad(offset), XSLOW_MEM_ACCESS),
                0x4210..=0x421F => (self.read_reg(offset), FAST_MEM_ACCESS),

                0x4300..=0x430A => (self.dma_channels[0].read((addr as u8) & 0xF), FAST_MEM_ACCESS),
                0x4310..=0x431A => (self.dma_channels[1].read((addr as u8) & 0xF), FAST_MEM_ACCESS),
                0x4320..=0x432A => (self.dma_channels[2].read((addr as u8) & 0xF), FAST_MEM_ACCESS),
                0x4330..=0x433A => (self.dma_channels[3].read((addr as u8) & 0xF), FAST_MEM_ACCESS),
                0x4340..=0x434A => (self.dma_channels[4].read((addr as u8) & 0xF), FAST_MEM_ACCESS),
                0x4350..=0x435A => (self.dma_channels[5].read((addr as u8) & 0xF), FAST_MEM_ACCESS),
                0x4360..=0x436A => (self.dma_channels[6].read((addr as u8) & 0xF), FAST_MEM_ACCESS),
                0x4370..=0x437A => (self.dma_channels[7].read((addr as u8) & 0xF), FAST_MEM_ACCESS),

                0x6000..=0xFFFF => self.cart.read(bank, offset),
                _               => (0, FAST_MEM_ACCESS),                                // Unmapped
            },
            0x40..=0x7D | 0xC0..=0xFF => self.cart.read(bank, offset),
            0x7E | 0x7F => (self.wram.read(addr - 0x7E0000), SLOW_MEM_ACCESS),
        }
    }

    pub fn write(&mut self, addr: u32, data: u8) -> usize {
        let bank = hi24!(addr);
        let offset = lo24!(addr);

        match bank {
            0x00..=0x3F | 0x80..=0xBF => match offset {
                0x0000..=0x1FFF => {self.wram.write(offset as u32, data); SLOW_MEM_ACCESS},

                0x2100..=0x2143 => {self.bus_b.write(lo!(offset), data); FAST_MEM_ACCESS},
                0x2180          => self.write(self.wram_addr, data),
                0x2181          => {self.wram_addr = set_lo24!(self.wram_addr, data); FAST_MEM_ACCESS},
                0x2182          => {self.wram_addr = set_mid24!(self.wram_addr, data); FAST_MEM_ACCESS},
                0x2183          => {self.wram_addr = set_hi24!(self.wram_addr, data); FAST_MEM_ACCESS},
                0x2100..=0x21FF => FAST_MEM_ACCESS,
                0x3000..=0x3FFF => FAST_MEM_ACCESS, // Extensions

                0x4000..=0x4015 |
                0x4017..=0x41FF => XSLOW_MEM_ACCESS,
                0x4016          => {self.bus_b.ppu.joypad_latch(); XSLOW_MEM_ACCESS},

                0x4200..=0x420d => {self.write_reg(offset, data); FAST_MEM_ACCESS},

                0x4300..=0x430A => {self.dma_channels[0].write((addr as u8) & 0xF, data); FAST_MEM_ACCESS},
                0x4310..=0x431A => {self.dma_channels[1].write((addr as u8) & 0xF, data); FAST_MEM_ACCESS},
                0x4320..=0x432A => {self.dma_channels[2].write((addr as u8) & 0xF, data); FAST_MEM_ACCESS},
                0x4330..=0x433A => {self.dma_channels[3].write((addr as u8) & 0xF, data); FAST_MEM_ACCESS},
                0x4340..=0x434A => {self.dma_channels[4].write((addr as u8) & 0xF, data); FAST_MEM_ACCESS},
                0x4350..=0x435A => {self.dma_channels[5].write((addr as u8) & 0xF, data); FAST_MEM_ACCESS},
                0x4360..=0x436A => {self.dma_channels[6].write((addr as u8) & 0xF, data); FAST_MEM_ACCESS},
                0x4370..=0x437A => {self.dma_channels[7].write((addr as u8) & 0xF, data); FAST_MEM_ACCESS},

                0x6000..=0xFFFF => self.cart.write(bank, offset, data),
                _               => FAST_MEM_ACCESS,  // Unmapped
            },
            0x40..=0x7D | 0xC0..=0xFF => self.cart.write(bank, offset, data),
            0x7E | 0x7F => {self.wram.write(addr - 0x7E0000, data); SLOW_MEM_ACCESS},
        }
    }

    // Clock the PPU and APU, and handle any signals coming from the PPU.
    pub fn clock(&mut self, cycles: usize) -> Option<Interrupt> {
        // TODO: clock APU.
        match self.bus_b.ppu.clock(cycles) {
            PPUSignal::NMI => Some(Interrupt::NMI),
            PPUSignal::IRQ => Some(Interrupt::IRQ),
            PPUSignal::HBlank => {
                if self.hdma_active != 0 {
                    self.hdma_transfer();
                }
                None
            },
            PPUSignal::Delay => {
                self.clock(PAUSE_LEN)
            },
            PPUSignal::None => None
        }
    }
}

// Internal
impl MemBus {
    fn make_cart(mut reader: BufReader<File>) -> Box<dyn Cart> {
        let mut buf = [0; 0x40];
        
        // Check for LoROM
        reader.seek(SeekFrom::Start(0x7FC0)).expect("Couldn't seek to cartridge header.");
        reader.read_exact(&mut buf).expect("Couldn't read cartridge header.");

        if (buf[0x15] & 0x21) == 0x20 {
            return Box::new(LoROM::new(reader, test_bit!(buf[0x15], 4, u8)));
        }

        // Check for HiROM
        reader.seek(SeekFrom::Start(0xFFC0)).expect("Couldn't seek to cartridge header.");
        reader.read_exact(&mut buf).expect("Couldn't read cartridge header.");

        if (buf[0x15] & 0x21) == 0x21 {
            return Box::new(HiROM::new(reader, test_bit!(buf[0x15], 4, u8)));
        } else {
            panic!("Unrecognised ROM: {:X}", buf[0x15]);
        }
    }

    // Internal status registers.
    fn read_reg(&mut self, addr: u16) -> u8 {
        match addr {
            0x4210 => self.bus_b.ppu.get_nmi_flag(),
            0x4211 => self.bus_b.ppu.get_irq_flag(),
            0x4212 => self.bus_b.ppu.get_status(), // PPU status
            0x4213 => 0, // IO port read
            0x4214 => lo!(self.div_result),
            0x4215 => hi!(self.div_result),
            0x4216 => lo!(self.mult_result),
            0x4217 => hi!(self.mult_result),
            0x4218..=0x421F => self.bus_b.ppu.read_joypad(addr),
            _ => 0,
        }
    }

    fn write_reg(&mut self, addr: u16, data: u8) {
        match addr {
            0x4200 => self.bus_b.ppu.set_int_enable(data),   // Enable IRQ
            0x4201 => {}, // IO port write
            0x4202 => self.mult_operand = data,
            0x4203 => self.mult_result = (self.mult_operand as u16) * (data as u16),
            0x4204 => self.div_operand = set_lo!(self.div_operand, data),
            0x4205 => self.div_operand = set_hi!(self.div_operand, data),
            0x4206 => {
                let divisor = data as u16;
                self.div_result = self.div_operand / divisor;
                self.mult_result = self.div_operand % divisor;
            },
            0x4207 => self.bus_b.ppu.set_h_timer_lo(data),
            0x4208 => self.bus_b.ppu.set_h_timer_hi(data),
            0x4209 => self.bus_b.ppu.set_v_timer_lo(data),
            0x420a => self.bus_b.ppu.set_v_timer_hi(data),
            0x420b => self.dma_transfer(data),
            0x420c => {
                self.hdma_active = data;
                for chan in 0..8 {
                    if test_bit!(self.hdma_active, chan, u8) {
                        self.dma_channels[chan].start_hdma();

                        let byte = self.read(self.dma_channels[chan].hdma_table_addr).0;
                        
                        if !self.dma_channels[chan].init_instr(byte) {
                            self.hdma_active ^= bit!(chan);
                        }
                    }
                }
            },
            0x420d => {}, // Fast ROM access speed
            _ => unreachable!(),
        }
    }

    // DMA
    // Keeps cycling until the transfer is done. This pauses the CPU operation.
    fn dma_transfer(&mut self, mut channels: u8) {
        for chan in 0..8 {

            while test_bit!(channels, chan, u8) {
                let (src_addr, dst_addr) = {
                    let channel = &mut self.dma_channels[chan];
                    if channel.control.contains(DMAControl::TRANSFER_DIR) {
                        (channel.get_b_bus_addr(), channel.get_a_bus_addr())
                    } else {
                        (channel.get_a_bus_addr(), channel.get_b_bus_addr())
                    }
                };

                match (self.dma_channels[chan].control & DMAControl::TRANSFER_MODE).bits() {
                    0 => {
                        let data = self.read(src_addr).0;
                        self.write(dst_addr, data);
                    },
                    1 => for i in 0..2 {
                        let data = self.read(src_addr + i).0;
                        self.write(dst_addr + i, data);
                    },
                    2 | 6 => for i in 0..2 {
                        let data = self.read(src_addr + i).0;
                        self.write(dst_addr, data);
                    },
                    3 | 7 => for i in 0..4 {
                        let data = self.read(src_addr + i).0;
                        self.write(dst_addr + (i / 2), data);
                    },
                    4 => for i in 0..4 {
                        let data = self.read(src_addr + i).0;
                        self.write(dst_addr + i, data);
                    },
                    5 => for i in 0..4 {
                        let data = self.read(src_addr + i).0;
                        self.write(dst_addr + (i % 2), data);
                    },
                    _ => unreachable!()
                }

                if self.dma_channels[chan].decrement_count() {
                    channels ^= bit!(chan);
                }

                self.clock(self.dma_channels[chan].get_cycles());
            }
        }
    }

    // Transfers a single block of HDMA data. Called during H-blank.
    fn hdma_transfer(&mut self) {
        for chan in 0..8 {
            if test_bit!(self.hdma_active, chan, u8) {
                // Get new instruction.
                if self.dma_channels[chan].get_line_count() == 0 {
                    self.dma_channels[chan].inc_table_addr();
                    // Finish channel.
                    let instr = self.read(self.dma_channels[chan].hdma_table_addr).0;
                    if !self.dma_channels[chan].init_instr(instr) {
                        self.hdma_active ^= bit!(chan);
                        continue;
                    } else if self.dma_channels[chan].once() {
                        self.hdma_line(chan);
                    }
                }

                // Run the instruction for each line.
                if !self.dma_channels[chan].once() {
                    self.hdma_line(chan);
                }

                self.dma_channels[chan].dec_line_count();
            }
        }
    }

    // A single HDMA line transfer.
    fn hdma_line(&mut self, chan: usize) {
        // Get bytes to write.
        match (self.dma_channels[chan].control & DMAControl::TRANSFER_MODE).bits() {
            0 => {
                let data = self.get_hdma_data(chan, 0);
                self.bus_b.write(self.dma_channels[chan].b_bus_addr, data);
            },
            1 => for i in 0_u8..2_u8 {
                let data = self.get_hdma_data(chan, i as u32);
                self.bus_b.write(self.dma_channels[chan].b_bus_addr + i, data);
            },
            2 | 6 => for i in 0_u8..2_u8 {
                let data = self.get_hdma_data(chan, i as u32);
                self.bus_b.write(self.dma_channels[chan].b_bus_addr, data);
            },
            3 | 7 => for i in 0_u8..4_u8 {
                let data = self.get_hdma_data(chan, i as u32);
                self.bus_b.write(self.dma_channels[chan].b_bus_addr + (i / 2), data);
            },
            4 => for i in 0_u8..4_u8 {
                let data = self.get_hdma_data(chan, i as u32);
                self.bus_b.write(self.dma_channels[chan].b_bus_addr + i, data);
            },
            5 => for i in 0_u8..4_u8 {
                let data = self.get_hdma_data(chan, i as u32);
                self.bus_b.write(self.dma_channels[chan].b_bus_addr + (i % 2), data);
            },
            _ => unreachable!()
        }

        self.clock(self.dma_channels[chan].get_cycles());
    }

    // Get HDMA data for a transfer.
    fn get_hdma_data(&mut self, chan: usize, offset: u32) -> u8 {
        if self.dma_channels[chan].control.contains(DMAControl::HDMA_INDIRECT) {
            let lo = self.read(self.dma_channels[chan].hdma_table_addr + (offset * 2)).0;
            let hi = self.read(self.dma_channels[chan].hdma_table_addr + (offset * 2) + 1).0;
            let indirect_addr = self.dma_channels[chan].indirect_table_addr(make16!(hi, lo));
            self.read(indirect_addr).0
        } else {
            self.read(self.dma_channels[chan].hdma_table_addr + offset).0
        }
    }
}

// Address Bus B, used for hardware registers.
struct AddrBusB {
    pub ppu:    PPU,
    apu:        APU
}

impl AddrBusB {
    fn new() -> Self {
        AddrBusB {
            ppu: PPU::new(),
            apu: APU::new()
        }
    }

    fn read(&mut self, addr: u8) -> u8 {
        match addr {
            0x37        => self.ppu.latch_hv(),
            0x34..=0x3F => self.ppu.read_mem(addr),
            0x40        => self.apu.read_port_0(),
            0x41        => self.apu.read_port_1(),
            0x42        => self.apu.read_port_2(),
            0x43        => self.apu.read_port_3(),
            _ => unreachable!()
        }
    }

    fn write(&mut self, addr: u8, data: u8) {
        match addr {
            0x00..=0x33 => self.ppu.write_mem(addr, data),
            0x40        => self.apu.write_port_0(data),
            0x41        => self.apu.write_port_1(data),
            0x42        => self.apu.write_port_2(data),
            0x43        => self.apu.write_port_3(data),
            _ => panic!("Tried to write silly shit: {:X} to {:X}", data, addr),
        }
    }
}