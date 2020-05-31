// Variable-length bit processing

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    struct VBD: u8 {
        const READ_MODE = bit!(7);          // 1 = auto-inc
        const DAT_LEN = bits![3, 2, 1, 0];  // Data length
    }
}

pub struct VarLengthProc {
    vbd:            VBD,
    data_len:       u8,
    rom_start_addr: u32,
}

impl VarLengthProc {
    pub fn new() -> Self {
        Self {
            vbd:            VBD::default(),
            data_len:       0,
            rom_start_addr: 0,
        }
    }

    pub fn write_vbd(&mut self, data: u8) {
        self.vbd = VBD::from_bits_truncate(data);
        self.data_len = match (self.vbd & VBD::DAT_LEN).bits() {
            0 => 16,
            x => x,
        };
    }

    pub fn write_addr_lo(&mut self, data: u8) {
        self.rom_start_addr = set_lo24(self.rom_start_addr, data);
    }

    pub fn write_addr_mid(&mut self, data: u8) {
        self.rom_start_addr = set_mid24(self.rom_start_addr, data);
    }

    pub fn write_addr_hi(&mut self, data: u8) {
        self.rom_start_addr = set_hi24(self.rom_start_addr, data);
    }
}