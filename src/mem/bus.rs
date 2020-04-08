// Address Buses A and B, and DMA operation.

use std::{
    io::{
        BufReader,
        Read,
        Seek,
        SeekFrom
    },
    fs::File,
    sync::{
        Arc, Mutex
    }
};

use crate::{
    common::Interrupt,
    constants::timing::*,
    video::{PPU, PPUSignal},
    audio::APU,
    joypad::{JoypadMem, Button}
};

use super::{
    RAM,
    SRAM,
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
    joypads:    JoypadMem,

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
    hdma_enable:    u8,
    hdma_active:    u8,
    dma_channels:   Vec<DMAChannel>,

    // Extensions
}

impl MemBus {
    pub fn new(cart_path: &str, save_path: &str) -> Self {
        // Open ROM file.
        let cart = MemBus::make_cart(cart_path, save_path);

        MemBus {
            bus_b:      AddrBusB::new(),
            joypads:    JoypadMem::new(),
            
            cart:       cart,
            wram:       RAM::new(0x20000),

            hdma_enable:    0,
            hdma_active:    0,
            dma_channels:   vec![DMAChannel::new(); 8],

            wram_addr:      0,
            mult_operand:   0xFF,
            div_operand:    0xFFFF,
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

                0x2100..=0x2143 => (self.bus_b.read(lo!(offset)), FAST_MEM_ACCESS),
                0x2180          => self.read_wram(),
                0x2100..=0x21FF => (0, FAST_MEM_ACCESS),
                0x3000..=0x3FFF => (0, FAST_MEM_ACCESS),                                // Extensions

                0x4000..=0x4015 |
                0x4000..=0x41FF => (self.joypads.read(offset), XSLOW_MEM_ACCESS),
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
                0x2180          => self.write_wram(data),
                0x2181          => {self.wram_addr = set_lo24!(self.wram_addr, data); FAST_MEM_ACCESS},
                0x2182          => {self.wram_addr = set_mid24!(self.wram_addr, data); FAST_MEM_ACCESS},
                0x2183          => {self.wram_addr = set_hi24!(self.wram_addr, data & 1); FAST_MEM_ACCESS},
                0x2100..=0x21FF => FAST_MEM_ACCESS,
                0x3000..=0x3FFF => FAST_MEM_ACCESS, // Extensions

                0x4000..=0x4015 |
                0x4017..=0x41FF => XSLOW_MEM_ACCESS,
                0x4016          => {self.joypads.latch_all(data); XSLOW_MEM_ACCESS},

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
    pub fn clock(&mut self, cycles: usize) -> Interrupt {
        self.bus_b.clock_apu(cycles);

        match self.bus_b.ppu.clock(cycles) {
            PPUSignal::Int(i) => {
                if i.intersects(Interrupt::NMI | Interrupt::VBLANK) {
                    self.joypads.prepare_read();
                }
                i
            }
            PPUSignal::HBlank => {
                if self.hdma_active != 0 {
                    self.hdma_transfer();
                }
                Interrupt::default()
            },
            PPUSignal::Delay => {
                self.bus_b.clock_apu(PAUSE_LEN);
                match self.bus_b.ppu.clock(PAUSE_LEN) {
                    PPUSignal::Int(Interrupt::IRQ) => Interrupt::IRQ, // This is the only one that should happen here.
                    PPUSignal::None => Interrupt::default(),
                    _ => unreachable!(),
                }
            },
            PPUSignal::FrameStart => {
                self.hdma_active = self.hdma_enable;
                for chan in 0..8 {
                    if test_bit!(self.hdma_active, chan, u8) {
                        self.dma_channels[chan].start_hdma();
                    }
                }
                Interrupt::default()
            },
            PPUSignal::None => Interrupt::default()
        }
    }

    // Enable or disable rendering video.
    pub fn enable_rendering(&mut self, enable: bool) {
        self.bus_b.ppu.enable_rendering(enable);
    }

    // Set buttons on the specified joypad.
    pub fn set_buttons(&mut self, button: Button, val: bool, joypad: usize) {
        self.joypads.set_buttons(button, val, joypad);
    }

    pub fn start_frame(&mut self, frame: Arc<Mutex<[u8]>>) {
        self.bus_b.ppu.start_frame(frame);
        self.cart.flush();
    }
}

// Internal
impl MemBus {
    fn make_cart(cart_path: &str, save_path: &str) -> Box<dyn Cart> {
        let rom_file = File::open(cart_path).expect(&format!("Couldn't open file {}", cart_path));
        //let rom_size = rom_file.metadata().expect("Couldn't get metadata for file.").len();

        let mut reader = BufReader::new(rom_file);
        
        let mut buf = [0; 0x40];
        
        // Check for LoROM
        reader.seek(SeekFrom::Start(0x7FC0)).expect("Couldn't seek to cartridge header.");
        reader.read_exact(&mut buf).expect("Couldn't read cartridge header.");

        if (buf[0x15] & 0xE9) == 0x20/* && (0x400 << buf[0x17]) == rom_size*/ {
            println!("LOROM {:X}: {}", buf[0x15], std::str::from_utf8(&buf[0..21]).unwrap());
            let save_file_size = std::cmp::min(0x400 << buf[0x18], 1024 * 512);    // TODO: check if there should be save data at all.
            let sram = SRAM::new(save_path, save_file_size).expect("Couldn't make save file.");
            return Box::new(LoROM::new(reader, sram, (buf[0x15] & 0x30) == 0x30));
        }

        // Check for HiROM
        reader.seek(SeekFrom::Start(0xFFC0)).expect("Couldn't seek to cartridge header.");
        reader.read_exact(&mut buf).expect("Couldn't read cartridge header.");

        if (buf[0x15] & 0xE9) == 0x21/* || (0x400 << buf[0x17]) == rom_size*/ {
            println!("HIROM {:X}: {}", buf[0x15], std::str::from_utf8(&buf[0..21]).unwrap());
            let save_file_size = std::cmp::min(0x400 << buf[0x18], 1024 * 256);    // TODO: check if there should be save data at all.
            let sram = SRAM::new(save_path, save_file_size).expect("Couldn't make save file.");
            return Box::new(HiROM::new(reader, sram, (buf[0x15] & 0x30) == 0x30));
        } else {
            panic!("Unrecognised ROM: {:X}", buf[0x15]);
        }
    }

    // WRAM access from special register.
    fn read_wram(&mut self) -> (u8, usize) {
        let data = self.wram.read(self.wram_addr);
        self.wram_addr = self.wram_addr.wrapping_add(1) & 0x1FFFF;
        (data, FAST_MEM_ACCESS)
    }

    fn write_wram(&mut self, data: u8) -> usize {
        self.wram.write(self.wram_addr, data);
        self.wram_addr = self.wram_addr.wrapping_add(1) & 0x1FFFF;
        FAST_MEM_ACCESS
    }

    // Internal status registers.
    fn read_reg(&mut self, addr: u16) -> u8 {
        match addr {
            0x4210 => self.bus_b.ppu.get_nmi_flag(),
            0x4211 => self.bus_b.ppu.get_irq_flag(),
            0x4212 => self.bus_b.ppu.get_status() | self.joypads.is_ready(), // PPU status
            0x4213 => 0, // IO port read
            0x4214 => lo!(self.div_result),
            0x4215 => hi!(self.div_result),
            0x4216 => lo!(self.mult_result),
            0x4217 => hi!(self.mult_result),
            0x4218..=0x421F => self.joypads.read(addr),
            _ => unreachable!(),
        }
    }

    fn write_reg(&mut self, addr: u16, data: u8) {
        match addr {
            0x4200 => { // Interrupt enable flags.
                self.bus_b.ppu.set_int_enable(data);
                self.joypads.enable_counter(data);
            },
            0x4201 => {}, // IO port write
            0x4202 => self.mult_operand = data,
            0x4203 => self.mult_result = (self.mult_operand as u16) * (data as u16),
            0x4204 => self.div_operand = set_lo!(self.div_operand, data),
            0x4205 => self.div_operand = set_hi!(self.div_operand, data),
            0x4206 => if data == 0 {
                self.div_result = 0xFFFF;
                self.mult_result = 0xC;
            } else {
                let divisor = data as u16;
                self.div_result = self.div_operand / divisor;
                self.mult_result = self.div_operand % divisor;
            },
            0x4207 => self.bus_b.ppu.set_h_timer_lo(data),
            0x4208 => self.bus_b.ppu.set_h_timer_hi(data),
            0x4209 => self.bus_b.ppu.set_v_timer_lo(data),
            0x420a => self.bus_b.ppu.set_v_timer_hi(data),
            0x420b => self.dma_transfer(data),
            0x420c => self.hdma_enable = data,
            0x420d => self.cart.set_rom_speed(data),
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

                        if self.dma_channels[chan].decrement_count() {
                            channels ^= bit!(chan);
                        }
                    },
                    1 => for i in 0..2 {
                        let data = self.read(src_addr + i).0;
                        self.write(dst_addr + i, data);

                        if self.dma_channels[chan].decrement_count() {
                            channels ^= bit!(chan);
                            break;
                        }
                    },
                    2 | 6 => for i in 0..2 {
                        let data = self.read(src_addr + i).0;
                        self.write(dst_addr, data);

                        if self.dma_channels[chan].decrement_count() {
                            channels ^= bit!(chan);
                            break;
                        }
                    },
                    3 | 7 => for i in 0..4 {
                        let data = self.read(src_addr + i).0;
                        self.write(dst_addr + (i / 2), data);

                        if self.dma_channels[chan].decrement_count() {
                            channels ^= bit!(chan);
                            break;
                        }
                    },
                    4 => for i in 0..4 {
                        let data = self.read(src_addr + i).0;
                        self.write(dst_addr + i, data);

                        if self.dma_channels[chan].decrement_count() {
                            channels ^= bit!(chan);
                            break;
                        }
                    },
                    5 => for i in 0..4 {
                        let data = self.read(src_addr + i).0;
                        self.write(dst_addr + (i % 2), data);

                        if self.dma_channels[chan].decrement_count() {
                            channels ^= bit!(chan);
                            break;
                        }
                    },
                    _ => unreachable!()
                }

                self.clock(self.dma_channels[chan].get_cycles());
            }
        }
    }

    // Transfers a single block of HDMA data. Called during H-blank.
    fn hdma_transfer(&mut self) {
        for chan in 0..8 {
            if test_bit!(self.hdma_active, chan, u8) {

                if self.dma_channels[chan].hdma_step_line() {
                    if self.dma_channels[chan].should_repeat() {
                        self.hdma_line(chan);
                    }
                } else {
                    // New instruction.
                    let instr = self.read(self.dma_channels[chan].get_hdma_table_addr()).0;
                    if self.dma_channels[chan].hdma_init_instr(instr) {
                        // Setup indirect address if necessary.
                        if self.dma_channels[chan].control.contains(DMAControl::HDMA_INDIRECT) {
                            let table_addr = self.dma_channels[chan].get_hdma_table_addr();
                            let lo = self.read(table_addr).0;
                            let hi = self.read(table_addr.wrapping_add(1)).0;
                            self.dma_channels[chan].set_indirect_table_addr(make16!(hi, lo));
                        }

                        self.hdma_line(chan);
                    } else {
                        self.hdma_active ^= bit!(chan);
                    }
                }
            }
        }
    }

    // A single HDMA line transfer.
    fn hdma_line(&mut self, chan: usize) {
        let src_addr = self.dma_channels[chan].get_data_addr();
        // Get bytes to write.
        match (self.dma_channels[chan].control & DMAControl::TRANSFER_MODE).bits() {
            0 => {
                let data = self.read(src_addr).0;
                self.bus_b.write(self.dma_channels[chan].b_bus_addr, data);
            },
            1 => for i in 0..2 {
                let data = self.read(src_addr + i).0;
                self.bus_b.write(self.dma_channels[chan].b_bus_addr + i as u8, data);
            },
            2 | 6 => for i in 0..2 {
                let data = self.read(src_addr + i).0;
                self.bus_b.write(self.dma_channels[chan].b_bus_addr, data);
            },
            3 | 7 => for i in 0..4 {
                let data = self.read(src_addr + i).0;
                self.bus_b.write(self.dma_channels[chan].b_bus_addr + ((i / 2) as u8), data);
            },
            4 => for i in 0..4 {
                let data = self.read(src_addr + i).0;
                self.bus_b.write(self.dma_channels[chan].b_bus_addr + i as u8, data);
            },
            5 => for i in 0..4 {
                let data = self.read(src_addr + i).0;
                self.bus_b.write(self.dma_channels[chan].b_bus_addr + ((i / 2) as u8), data);
            },
            _ => unreachable!()
        }

        self.clock(self.dma_channels[chan].get_cycles());
    }
}

// Amount of cycles to wait before telling the APU to clock.
const APU_CYCLE_BATCH: usize = 100;

// Address Bus B, used for hardware registers.
struct AddrBusB {
    ppu:        PPU,
    apu:        APU,

    apu_cycle_count:    usize,

    open_bus:   u8
}

impl AddrBusB {
    fn new() -> Self {
        AddrBusB {
            ppu: PPU::new(),
            apu: APU::new(),

            apu_cycle_count:    0,

            open_bus:   0,
        }
    }

    fn read(&mut self, addr: u8) -> u8 {
        match addr {
            0x37        => self.ppu.latch_hv(),
            0x34..=0x3F => self.ppu.read_mem(addr),
            0x40..=0x7F => match addr % 4 {
                0   => self.apu.read_port_0(),
                1   => self.apu.read_port_1(),
                2   => self.apu.read_port_2(),
                3   => self.apu.read_port_3(),
                _   => unreachable!(),
            },
            _ => self.open_bus//unreachable!("Reading from open bus: {:X}", addr)
        }
    }

    fn write(&mut self, addr: u8, data: u8) {
        match addr {
            0x00..=0x33 => self.ppu.write_mem(addr, data),
            0x40..=0x7F => match addr % 4 {
                0   => self.apu.write_port_0(data),
                1   => self.apu.write_port_1(data),
                2   => self.apu.write_port_2(data),
                3   => self.apu.write_port_3(data),
                _   => unreachable!(),
            },
            0x34..=0x3F => {},
            _ => {}//panic!("Tried to write silly shit: {:X} to {:X}", data, addr),
        }
        self.open_bus = data;
    }

    // Increase APU cycle count and send message if threshold is reached.
    fn clock_apu(&mut self, cycles: usize) {
        self.apu_cycle_count += cycles;

        if self.apu_cycle_count >= APU_CYCLE_BATCH {
            self.apu.clock(self.apu_cycle_count);

            self.apu_cycle_count = 0;
        }
    }
}