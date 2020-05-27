// ROM header. Contains metadata about the ROM.

use std::{
    io::{
        BufReader,
        Read,
        Seek,
        SeekFrom
    },
    fs::File
};

const ROM_MAPPING_MASK: u8 = 0xE9;

pub struct ROMHeader {
    data: [u8; 40]
}

impl ROMHeader {
    pub fn new() -> Self {
        Self {
            data: [0; 40]
        }
    }

    /// Set the header to the LOROM position, and check if it is a lorom header.
    pub fn try_lo(&mut self, reader: &mut BufReader<File>) -> bool {
        const LO_ROM_HEADER_START: u64 = 0x7FC0;
        const LO_ROM: u8 = 0x20;

        reader.seek(SeekFrom::Start(LO_ROM_HEADER_START)).expect("Couldn't seek to cartridge header.");
        reader.read_exact(&mut self.data).expect("Couldn't read cartridge header.");

        (self.rom_mapping() & ROM_MAPPING_MASK) == LO_ROM
    }

    /// Set the header to the HIROM position, and check if it is a hirom header.
    pub fn try_hi(&mut self, reader: &mut BufReader<File>) -> bool {
        const HI_ROM_HEADER_START: u64 = 0xFFC0;
        const HI_ROM: u8 = 0x21;

        reader.seek(SeekFrom::Start(HI_ROM_HEADER_START)).expect("Couldn't seek to cartridge header.");
        reader.read_exact(&mut self.data).expect("Couldn't read cartridge header.");

        (self.rom_mapping() & ROM_MAPPING_MASK) == HI_ROM
    }

    // Header metadata.

    /// Name of the game.
    pub fn rom_name(&self) -> String {
        use std::str::FromStr;
        String::from_str(std::str::from_utf8(&self.data[0..21]).unwrap()).unwrap()
    }

    /// Mapping type.
    pub fn rom_mapping(&self) -> u8 {
        self.data[0x15]
    }

    /// ROM type
    pub fn rom_type(&self) -> ROMType {
        self.data[0x16].into()
    }

    /// ROM size in bytes.
    pub fn rom_size(&self) -> usize {
        0x400 << self.data[0x17]
    }

    /// SRAM size in bytes.
    pub fn sram_size(&self) -> usize {
        if self.rom_type().has_sram() {
            let indicated_size = 0x400 << self.data[0x18];
            std::cmp::min(indicated_size, 1024 * 512)
        } else {
            0
        }
    }

    /// Check if this ROM has fast RAM.
    pub fn fast_rom(&self) -> bool {
        (self.rom_mapping() & 0x30) == 0x30
    }
}

#[derive(Debug)]
pub enum EnhancementChip {
    DSP,
    SuperFX,
    OBC1,
    SA1,
    Other,
    Custom,
    Unknown
}

#[derive(Clone, Copy)]
pub struct ROMType {
    rom_type: u8
}

impl From<u8> for ROMType {
    fn from(val: u8) -> Self {
        Self {
            rom_type: val
        }
    }
}

impl ROMType {
    pub fn has_sram(self) -> bool {
        let lower_nybble = lo_nybble!(self.rom_type);
        lower_nybble == 2 || lower_nybble == 5 || lower_nybble == 6
    }

    pub fn enhancement_chip(self) -> Option<EnhancementChip> {
        use EnhancementChip::*;
        if lo_nybble!(self.rom_type) > 2 {
            Some(match hi_nybble!(self.rom_type) {
                0x0 => DSP,
                0x1 => SuperFX,
                0x2 => OBC1,
                0x3 => SA1,
                0xE => Other,
                0xF => Custom,
                _ => Unknown
            })
        } else {
            None
        }
    }
}