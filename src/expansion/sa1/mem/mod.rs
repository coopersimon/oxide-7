// Memory bus attached to SA-1.

mod arith;
mod timer;

use arith::Arithmetic;
use timer::Timer;
//use varproc::VarLengthProc;

use crate::{
    constants::int,
    common::Interrupt,
    mem::{
        MemBus,
        RAM,
        rom::ROM
    }
};

const RST_VECTOR_LO: u16 = int::RESET_VECTOR_EMU as u16;
const RST_VECTOR_HI: u16 = (int::RESET_VECTOR_EMU as u16) + 1;
const NMI_VECTOR_LO: u16 = int::NMI_VECTOR as u16;
const NMI_VECTOR_HI: u16 = (int::NMI_VECTOR as u16) + 1;
const IRQ_VECTOR_LO: u16 = int::IRQ_VECTOR as u16;
const IRQ_VECTOR_HI: u16 = (int::IRQ_VECTOR as u16) + 1;

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    struct SNESControl: u8 {
        const IRQ = bit!(7);                // IRQ from SA-1
        const IRQ_VEC = bit!(6);            // IRQ Vector for SNES
        const DMA_IRQ = bit!(5);            // IRQ ready from character conversion
        const NMI_VEC = bit!(4);            // NMI Vector for SNES
        const MESSAGE = bits![3, 2, 1, 0];  // Message from SA-1 to SNES
    }
}

bitflags! {
    #[derive(Default)]
    struct SA1Control: u8 {
        const IRQ = bit!(7);                // IRQ from SNES
        const WAIT = bit!(6);               // Wait from SNES
        const RST = bit!(5);                // RST from SNES
        const NMI = bit!(4);                // NMI from SNES
        const MESSAGE = bits![3, 2, 1, 0];  // Message from SNES to SA-1
    }
}

bitflags! {
    #[derive(Default)]
    struct SNESCPUInt: u8 {
        const SA1_IRQ = bit!(7);
        const DMA_IRQ = bit!(5);
    }
}

bitflags! {
    #[derive(Default)]
    struct SA1CPUInt: u8 {
        const SNES_IRQ = bit!(7);
        const TIMER_IRQ = bit!(6);
        const DMA_IRQ = bit!(5);
        const NMI = bit!(4);
    }
}

bitflags! {
    #[derive(Default)]
    struct DMAControl: u8 {
        const ENABLE = bit!(7);
        const PRIORITY = bit!(6);
        const CHAR_CONV_ENABLE = bit!(5);
        const CHAR_CONV_TYPE = bit!(4);
        const DST_DEVICE = bit!(2);
        const SRC_DEVICE = bits![1, 0];
    }
}

bitflags! {
    #[derive(Default)]
    struct CDMA: u8 {
        const TERMINATE = bit!(7);
        const VIRTUAL_VRAM_WIDTH = bits![4, 3, 2];
        const COLOUR_DEPTH = bits![1, 0];
    }
}


pub struct SA1Bus {
    // Memory
    rom:    ROM,
    // SRAM?
    iram:   RAM,
    bwram:  RAM,

    // Banking
    rom_bank_c: u8,
    rom_bank_d: u8,
    rom_bank_e: u8,
    rom_bank_f: u8,
    bwram_bitmap: u8,

    // DMA
    dma_control: DMAControl,
    cdma_params: CDMA,
    dma_src_addr: u32,
    dma_dst_addr: u32,
    dma_counter: u16,
    dma_bitmap_regs: [u8; 16],

    // SA-1 Status
    sa1_cpu_control: SA1Control,
    sa1_int_enable: SA1CPUInt,
    sa1_int_pending: SA1CPUInt,
    sa1_rst_vector: u16,
    sa1_nmi_vector: u16,
    sa1_irq_vector: u16,
    sa1_bw_map: u8,
    sa1_bw_write_enable: u8,
    sa1_iram_write: u8,

    // SNES Status
    //cpu_int: Interrupt,         // Pending interrupts from SNES CPU.
    snes_cpu_control: SNESControl,
    snes_int_enable: SNESCPUInt,
    //snes_int_pending: SNESCPUInt,
    snes_nmi_vector: u16,       // Custom NMI Vector
    snes_irq_vector: u16,       // Custom IRQ Vector
    snes_bw_map: u8,            // BW-RAM mapped to SNES CPU.
    snes_bw_write_enable: u8,   // Bit 7: enable writing to BWRAM
    snes_iram_write: u8,        // Each bit protects a 256-byte area of IRAM

    // Misc
    lo_rom:     bool,           // ROM uses a lo or hi mapping.
    timer:      Timer,
    arith:      Arithmetic,
    cycle_count:    usize,
    reset_latch: bool,
    wait_latch: bool,
}

impl SA1Bus {
    pub fn new(rom: ROM, lo_rom: bool) -> Self {
        Self {
            rom:        rom,
            iram:       RAM::new(0x800),
            bwram:      RAM::new(0x20000),

            rom_bank_c: 0,
            rom_bank_d: 1,
            rom_bank_e: 2,
            rom_bank_f: 3,
            bwram_bitmap: 0,
            dma_bitmap_regs: [0; 16],

            dma_control: DMAControl::default(),
            cdma_params: CDMA::default(),
            dma_src_addr: 0,
            dma_dst_addr: 0,
            dma_counter: 0,

            sa1_cpu_control: SA1Control::default(),
            sa1_int_enable: SA1CPUInt::default(),
            sa1_int_pending: SA1CPUInt::default(),
            sa1_rst_vector: 0,
            sa1_nmi_vector: 0,
            sa1_irq_vector: 0,
            sa1_bw_map: 0,
            sa1_bw_write_enable: 0,
            sa1_iram_write: 0,

            //cpu_int:    Interrupt::default(),
            snes_cpu_control: SNESControl::default(),
            snes_int_enable: SNESCPUInt::default(),
            //snes_int_pending: SNESCPUInt::default(),
            snes_nmi_vector: NMI_VECTOR_LO,
            snes_irq_vector: IRQ_VECTOR_LO,
            snes_bw_map: 0,
            snes_bw_write_enable: 0,
            snes_iram_write: 0,

            lo_rom:     lo_rom,
            timer:      Timer::new(),
            arith:      Arithmetic::new(),
            cycle_count:    0,
            reset_latch:    true,
            wait_latch: false,
        }
    }

    pub fn snes_read(&mut self, bank: u8, addr: u16) -> u8 {
        match bank {
            0x00 => match addr {
                NMI_VECTOR_LO if self.snes_cpu_control.contains(SNESControl::NMI_VEC) => lo!(self.snes_nmi_vector),
                NMI_VECTOR_HI if self.snes_cpu_control.contains(SNESControl::NMI_VEC) => hi!(self.snes_nmi_vector),
                IRQ_VECTOR_LO if self.snes_cpu_control.contains(SNESControl::IRQ_VEC) => lo!(self.snes_irq_vector),
                IRQ_VECTOR_HI if self.snes_cpu_control.contains(SNESControl::IRQ_VEC) => hi!(self.snes_irq_vector),
                0x2200..=0x23FF => self.read_snes_port(addr),
                0x3000..=0x37FF => self.iram.read((addr - 0x3000) as u32),
                0x6000..=0x7FFF => self.read_snes_mapped_bwram(addr - 0x6000),
                0x8000..=0xFFFF => self.read_rom(bank, addr),
                _ => 0
            },
            0x01..=0x3F | 0x80..=0xBF => match addr {
                0x2200..=0x23FF => self.read_snes_port(addr),
                0x3000..=0x37FF => self.iram.read((addr - 0x3000) as u32),
                0x6000..=0x7FFF => self.read_snes_mapped_bwram(addr - 0x6000),
                0x8000..=0xFFFF => self.read_rom(bank, addr),
                _ => 0
            },
            0x40..=0x4F => {
                let bank_mod = (bank - 0x40) as u32;
                let bw_addr = (bank_mod * 0x10000) + (addr as u32);
                self.bwram.read(bw_addr)
            },
            0x40..=0x7F | 0xC0..=0xFF => self.read_rom(bank, addr),
        }
    }

    pub fn snes_write(&mut self, bank: u8, addr: u16, data: u8) {
        match bank {
            0x00..=0x3F | 0x80..=0xBF => match addr {
                0x2200..=0x23FF => self.write_snes_port(addr, data),
                0x3000..=0x37FF => self.write_snes_iram(addr - 0x3000, data),
                0x6000..=0x7FFF => self.write_snes_mapped_bwram(addr - 0x6000, data),
                _ => {}
            },
            0x40..=0x4F if test_bit!(self.snes_bw_write_enable, 7, u8) => {
                let bank_mod = (bank - 0x40) as u32;
                let bw_addr = (bank_mod * 0x10000) + (addr as u32);
                self.bwram.write(bw_addr, data)
            },
            _ => {}
        }
    }

    pub fn check_snes_interrupts(&mut self) -> Interrupt {
        if (self.snes_cpu_control.contains(SNESControl::IRQ) && self.snes_int_enable.contains(SNESCPUInt::SA1_IRQ)) ||
            (self.snes_cpu_control.contains(SNESControl::DMA_IRQ) && self.snes_int_enable.contains(SNESCPUInt::DMA_IRQ)) {
            Interrupt::IRQ
        } else {
            Interrupt::default()
        }
    }

    pub fn get_cycle_count(&mut self) -> usize {
        std::mem::replace(&mut self.cycle_count, 0)
    }
}

impl MemBus for SA1Bus {
    fn read(&mut self, addr: u32) -> (u8, usize) {
        let bank = hi24!(addr);
        let offset = lo24!(addr);

        match bank {
            0x00 => match offset {
                NMI_VECTOR_LO => (lo!(self.sa1_nmi_vector), 2),
                NMI_VECTOR_HI => (hi!(self.sa1_nmi_vector), 2),
                RST_VECTOR_LO => (lo!(self.sa1_rst_vector), 2),
                RST_VECTOR_HI => (hi!(self.sa1_rst_vector), 2),
                IRQ_VECTOR_LO => (lo!(self.sa1_irq_vector), 2),
                IRQ_VECTOR_HI => (hi!(self.sa1_irq_vector), 2),
                0x0000..=0x07FF => (self.iram.read(offset as u32), 2),
                0x2200..=0x23FF => (self.read_sa1_port(offset), 2),
                0x3000..=0x37FF => (self.iram.read((offset - 0x3000) as u32), 2),
                0x6000..=0x7FFF => (self.read_sa1_mapped_bwram(offset - 0x6000), 2),
                0x8000..=0xFFFF => (self.read_rom(bank, offset), 2),
                _ => (0, 2),
            },
            0x01..=0x3F | 0x80..=0xBF => match offset {
                0x0000..=0x07FF => (self.iram.read(offset as u32), 2),
                0x2200..=0x23FF => (self.read_sa1_port(offset), 2),
                0x3000..=0x37FF => (self.iram.read((offset - 0x3000) as u32), 2),
                0x6000..=0x7FFF => (self.read_sa1_mapped_bwram(offset - 0x6000), 2),
                0x8000..=0xFFFF => (self.read_rom(bank, offset), 2),
                _ => (0, 2),
            },
            0x40..=0x4F => (self.bwram.read(addr - 0x40_0000), 2),
            0x60..=0x6F => (self.read_sa1_bitmapped_bwram(addr - 0x60_0000), 2),
            0x40..=0x7F | 0xC0..=0xFF => (self.read_rom(bank, offset), 2),
        }
    }

    fn write(&mut self, addr: u32, data: u8) -> usize {
        let bank = hi24!(addr);
        let offset = lo24!(addr);

        match bank {
            0x00..=0x3F => match offset {
                0x0000..=0x07FF => self.write_sa1_iram(offset, data),
                0x2200..=0x23FF => self.write_sa1_port(offset, data),
                0x3000..=0x37FF => self.write_sa1_iram(offset - 0x3000, data),
                0x6000..=0x7FFF => self.write_sa1_mapped_bwram(offset - 0x6000, data),
                _ => {},
            },
            0x40..=0x4F if test_bit!(self.sa1_bw_write_enable, 7, u8) => self.bwram.write(addr - 0x40_0000, data),
            0x60..=0x6F => self.write_sa1_bitmapped_bwram(addr - 0x60_0000, data),
            _ => {},
        }

        2
    }

    // Clocking from SA-1 CPU.
    fn clock(&mut self, cycles: usize) -> Interrupt {
        let mut ret = Interrupt::default();

        // Timer
        if self.timer.clock(cycles) && self.sa1_int_enable.contains(SA1CPUInt::TIMER_IRQ) {
            self.sa1_int_pending |= SA1CPUInt::TIMER_IRQ;
            ret |= Interrupt::IRQ;
        }

        // DMA

        // Check SNES-sourced interrupts
        if self.sa1_cpu_control.contains(SA1Control::NMI) && self.sa1_int_enable.contains(SA1CPUInt::NMI) {
            self.sa1_cpu_control.remove(SA1Control::NMI);   // TODO: how to stop this repeatedly being called?
            ret |= Interrupt::NMI;
        }
        if !self.reset_latch && self.sa1_cpu_control.contains(SA1Control::RST) {
            self.reset_latch = true;
        }
        if self.reset_latch && !self.sa1_cpu_control.contains(SA1Control::RST) {
            self.reset_latch = false;
            ret |= Interrupt::RESET;
        }
        if self.sa1_cpu_control.contains(SA1Control::WAIT) && !self.wait_latch {
            ret |= Interrupt::WAIT;
        }
        if self.wait_latch && !self.sa1_cpu_control.contains(SA1Control::WAIT) {    // TODO: better way of ending wait?
            ret |= Interrupt::WAIT;
        }
        if self.sa1_cpu_control.contains(SA1Control::IRQ) && self.sa1_int_enable.contains(SA1CPUInt::SNES_IRQ) {
            ret |= Interrupt::IRQ;
        }

        self.cycle_count += cycles;

        ret
    }
}

// Internal: SNES side
impl SA1Bus {
    fn read_snes_port(&mut self, addr: u16) -> u8 {
        //println!("Reading from {:X}", addr);
        match addr {
            0x2300 => self.snes_cpu_control.bits(),
            0x230E => 0,    // SNES VC
            _ => 0,
        }
    }

    fn write_snes_port(&mut self, addr: u16, data: u8) {
        //println!("Writing {:X} to {:X}", data, addr);
        match addr {
            0x2200 => self.sa1_cpu_control = SA1Control::from_bits_truncate(data),
            0x2201 => self.snes_int_enable = SNESCPUInt::from_bits_truncate(data),
            0x2202 => self.clear_snes_ints(data),
            0x2203 => self.sa1_rst_vector = set_lo!(self.sa1_rst_vector, data),
            0x2204 => self.sa1_rst_vector = set_hi!(self.sa1_rst_vector, data),
            0x2205 => self.sa1_nmi_vector = set_lo!(self.sa1_nmi_vector, data),
            0x2206 => self.sa1_nmi_vector = set_hi!(self.sa1_nmi_vector, data),
            0x2207 => self.sa1_irq_vector = set_lo!(self.sa1_irq_vector, data),
            0x2208 => self.sa1_irq_vector = set_hi!(self.sa1_irq_vector, data),

            0x2220 => self.rom_bank_c = data,
            0x2221 => self.rom_bank_d = data,
            0x2222 => self.rom_bank_e = data,
            0x2223 => self.rom_bank_f = data,
            0x2224 => self.snes_bw_map = data & 0x1F,
            0x2226 => self.snes_bw_write_enable = data,
            0x2228 => {},   // BW-RAM write-protected area (default = 0xFF)
            0x2229 => self.snes_iram_write = data,

            0x2231 => self.cdma_params = CDMA::from_bits_truncate(data),
            0x2232 => self.dma_src_addr = set_lo24!(self.dma_src_addr, data),
            0x2233 => self.dma_src_addr = set_mid24!(self.dma_src_addr, data),
            0x2234 => self.dma_src_addr = set_hi24!(self.dma_src_addr, data),
            0x2235 => self.dma_dst_addr = set_lo24!(self.dma_dst_addr, data),
            0x2236 => self.write_dma_dst_addr_mid(data),
            0x2237 => self.write_dma_dst_addr_hi(data),

            _ => {}
        }
    }

    fn clear_snes_ints(&mut self, data: u8) {
        let to_clear = SNESCPUInt::from_bits_truncate(data);
        if to_clear.contains(SNESCPUInt::SA1_IRQ) {
            self.snes_cpu_control.remove(SNESControl::IRQ);
        }
        if to_clear.contains(SNESCPUInt::DMA_IRQ) {
            self.snes_cpu_control.remove(SNESControl::DMA_IRQ);
        }
    }

    fn write_snes_iram(&mut self, addr: u16, data: u8) {
        let region = addr >> 8;
        if test_bit!(self.snes_iram_write, region, u8) {
            self.iram.write(addr as u32, data);
        }
    }

    fn read_snes_mapped_bwram(&mut self, addr: u16) -> u8 {
        let bw_mapped_offset = (self.snes_bw_map as u32) * 0x2000;
        let bw_mapped_addr = bw_mapped_offset + (addr as u32);
        self.bwram.read(bw_mapped_addr)
    }

    fn write_snes_mapped_bwram(&mut self, addr: u16, data: u8) {
        if test_bit!(self.snes_bw_write_enable, 7, u8) {
            let bw_mapped_offset = (self.snes_bw_map as u32) * 0x2000;
            let bw_mapped_addr = bw_mapped_offset + (addr as u32);
            self.bwram.write(bw_mapped_addr, data);
        }
    }
}

// Internal: SA-1 side
impl SA1Bus {
    fn read_sa1_port(&mut self, addr: u16) -> u8 {
        //println!("SA Reading from {:X}", addr);
        match addr {
            0x2301 => {
                let message = self.sa1_cpu_control.bits() & 0xF;
                let ints = self.sa1_int_pending.bits() & 0xF0;
                ints | message
            },
            0x2302 => self.timer.read_h_lo(),
            0x2303 => self.timer.read_h_hi(),
            0x2304 => self.timer.read_v_lo(),
            0x2305 => self.timer.read_v_hi(),

            0x2306 => self.arith.read_result_0(),
            0x2307 => self.arith.read_result_1(),
            0x2308 => self.arith.read_result_2(),
            0x2309 => self.arith.read_result_3(),
            0x230A => self.arith.read_result_4(),
            0x230B => self.arith.read_ovf(),

            0x230C => 0,    // Var-length port
            0x230D => 0,    // Var-length port

            _ => 0,
        }
    }

    fn write_sa1_port(&mut self, addr: u16, data: u8) {
        //println!("SA Writing {:X} to {:X}", data, addr);
        match addr {
            0x2209 => {
                let dma_irq = self.snes_cpu_control.contains(SNESControl::DMA_IRQ);
                self.snes_cpu_control = SNESControl::from_bits_truncate(data);
                self.snes_cpu_control.set(SNESControl::DMA_IRQ, dma_irq);
            },
            0x220A => self.sa1_int_enable = SA1CPUInt::from_bits_truncate(data),
            0x220B => self.clear_sa1_ints(data),
            0x220C => self.snes_nmi_vector = set_lo!(self.snes_nmi_vector, data),
            0x220D => self.snes_nmi_vector = set_hi!(self.snes_nmi_vector, data),
            0x220E => self.snes_irq_vector = set_lo!(self.snes_irq_vector, data),
            0x220F => self.snes_irq_vector = set_hi!(self.snes_irq_vector, data),
            0x2210 => self.timer.write_control(data),
            0x2211 => self.timer.restart(),
            0x2212 => self.timer.write_h_lo(data),
            0x2213 => self.timer.write_h_hi(data),
            0x2214 => self.timer.write_v_lo(data),
            0x2215 => self.timer.write_v_hi(data),

            0x2225 => self.sa1_bw_map = data,
            0x2227 => self.sa1_bw_write_enable = data,
            0x222A => self.sa1_iram_write = data,

            0x2230 => self.dma_control = DMAControl::from_bits_truncate(data),
            0x2231 => self.cdma_params = CDMA::from_bits_truncate(data),
            0x2232 => self.dma_src_addr = set_lo24!(self.dma_src_addr, data),
            0x2233 => self.dma_src_addr = set_mid24!(self.dma_src_addr, data),
            0x2234 => self.dma_src_addr = set_hi24!(self.dma_src_addr, data),
            0x2235 => self.dma_dst_addr = set_lo24!(self.dma_dst_addr, data),
            0x2236 => self.write_dma_dst_addr_mid(data),
            0x2237 => self.write_dma_dst_addr_hi(data),
            0x2238 => self.dma_counter = set_lo!(self.dma_counter, data),
            0x2239 => self.dma_counter = set_hi!(self.dma_counter, data),
            0x2240..=0x224F => self.dma_bitmap_regs[(addr & 0xF) as usize] = data,

            0x223F => self.bwram_bitmap = data,

            0x2250 => self.arith.write_control(data),
            0x2251 => self.arith.write_param_a_lo(data),
            0x2252 => self.arith.write_param_a_hi(data),
            0x2253 => self.arith.write_param_b_lo(data),
            0x2254 => self.arith.write_param_b_hi(data),

            0x2258 => {},   // VBD
            0x2259 => {},   // Var length ADDR
            0x225A => {},
            0x225B => {},

            _ => {}
        }
    }

    fn clear_sa1_ints(&mut self, data: u8) {
        let to_clear = SA1CPUInt::from_bits_truncate(data);
        self.sa1_int_pending.remove(to_clear);
        if to_clear.contains(SA1CPUInt::SNES_IRQ) {
            self.sa1_cpu_control.remove(SA1Control::IRQ);
        }
        if to_clear.contains(SA1CPUInt::NMI) {
            self.sa1_cpu_control.remove(SA1Control::NMI);
        }
    }

    fn write_sa1_iram(&mut self, addr: u16, data: u8) {
        let region = addr >> 8;
        if test_bit!(self.sa1_iram_write, region, u8) {
            self.iram.write(addr as u32, data);
        }
    }

    fn read_sa1_mapped_bwram(&mut self, addr: u16) -> u8 {
        let bw_mapped_offset = ((self.sa1_bw_map & 0x7F) as u32) * 0x2000;
        let bw_mapped_addr = bw_mapped_offset + (addr as u32);
        self.bwram.read(bw_mapped_addr)
    }

    fn write_sa1_mapped_bwram(&mut self, addr: u16, data: u8) {
        if test_bit!(self.sa1_bw_write_enable, 7, u8) {
            let bw_mapped_offset = (self.sa1_bw_map as u32) * 0x2000;
            let bw_mapped_addr = bw_mapped_offset + (addr as u32);
            self.bwram.write(bw_mapped_addr, data);
        }
    }

    fn read_sa1_bitmapped_bwram(&mut self, addr: u32) -> u8 {
        if test_bit!(self.bwram_bitmap, 7, u8) {
            let real_addr = addr / 2;
            let shift = (addr % 2) * 4;
            (self.bwram.read(real_addr) >> shift) & 0xF
        } else {
            let real_addr = addr / 4;
            let shift = (addr % 4) * 2;
            (self.bwram.read(real_addr) >> shift) & 0x3
        }
    }

    fn write_sa1_bitmapped_bwram(&mut self, addr: u32, data: u8) {
        if test_bit!(self.sa1_bw_write_enable, 7, u8) {
            if test_bit!(self.bwram_bitmap, 7, u8) {
                let data_in = data & 0xF;
                let real_addr = addr / 2;
                let current_data = self.bwram.read(real_addr);
                match addr % 2 {
                    0 => self.bwram.write(real_addr, (current_data & 0xF0) | data_in),
                    _ => self.bwram.write(real_addr, (current_data & 0x0F) | data_in << 4),
                }
            } else {
                let data_in = data & 0x3;
                let real_addr = addr / 4;
                let current_data = self.bwram.read(real_addr);
                match addr % 4 {
                    0 => self.bwram.write(real_addr, (current_data & 0xFC) | data_in),
                    1 => self.bwram.write(real_addr, (current_data & 0xF3) | data_in << 2),
                    2 => self.bwram.write(real_addr, (current_data & 0xCF) | data_in << 4),
                    _ => self.bwram.write(real_addr, (current_data & 0x3F) | data_in << 6),
                }
            }
        }
    }
}

// Internal: other
impl SA1Bus {
    fn read_rom(&mut self, bank: u8, addr: u16) -> u8 {
        let mapped_bank = match bank {
            0x00..=0x1F => if test_bit!(self.rom_bank_c, 7, u8) {
                map_bank(bank, self.rom_bank_c)
            } else {
                let hi = hi_nybble!(bank);
                map_bank(bank, hi)
            },
            0x20..=0x3F => if test_bit!(self.rom_bank_d, 7, u8) {
                map_bank(bank, self.rom_bank_d)
            } else {
                let hi = hi_nybble!(bank);
                map_bank(bank, hi)
            },
            //0x40..=0x7F => if self.lo_rom {bank} else {bank % 0x40},
            0x80..=0x9F => if test_bit!(self.rom_bank_e, 7, u8) {
                map_bank(bank, self.rom_bank_e)
            } else {
                let hi = hi_nybble!(bank) & 0x3;
                map_bank(bank, 4 + hi)
            },
            0xA0..=0xBF => if test_bit!(self.rom_bank_f, 7, u8) {
                map_bank(bank, self.rom_bank_f)
            } else {
                let hi = hi_nybble!(bank) & 0x3;
                map_bank(bank, 4 + hi)
            },
            0xC0..=0xCF => map_hirom_bank(bank, self.rom_bank_c, self.lo_rom, addr),
            0xD0..=0xDF => map_hirom_bank(bank, self.rom_bank_d, self.lo_rom, addr),
            0xE0..=0xEF => map_hirom_bank(bank, self.rom_bank_e, self.lo_rom, addr),
            0xF0..=0xFF => map_hirom_bank(bank, self.rom_bank_f, self.lo_rom, addr),
            _ => 0,
        };

        self.rom.read(mapped_bank, if self.lo_rom {addr % 0x8000} else {addr})
    }

    // Starts the transfer in the case of I-RAM
    fn write_dma_dst_addr_mid(&mut self, data: u8) {
        self.dma_dst_addr = set_mid24!(self.dma_dst_addr, data);
        if !self.dma_control.contains(DMAControl::DST_DEVICE) {
            self.dma_transfer();
        }
    }

    // Starts the transfer in the case of BW-RAM
    fn write_dma_dst_addr_hi(&mut self, data: u8) {
        self.dma_dst_addr = set_hi24!(self.dma_dst_addr, data);
        if self.dma_control.contains(DMAControl::DST_DEVICE) {
            self.dma_transfer();
        }
    }

    fn dma_transfer(&mut self) {
        for _i in 0..self.dma_counter {
            // transfer
        }

        self.dma_counter = 0;
    }
}

fn map_bank(in_bank: u8, mapping: u8) -> u8 {
    let lo = lo_nybble!(in_bank);
    let hi = (mapping % 8) << 4;
    hi | lo
}

fn map_hirom_bank(in_bank: u8, mapping: u8, lo_rom: bool, bank_addr: u16) -> u8 {
    let mapped_bank = map_bank(in_bank, mapping);
    if lo_rom {
        if bank_addr >= 0x8000 {
            (mapped_bank * 2) + 1
        } else {
            mapped_bank * 2
        }
    } else {
        mapped_bank
    }
}