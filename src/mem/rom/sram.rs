use std::{
    io::{
        BufReader,
        BufWriter,
        Read,
        Write,
        Seek,
        SeekFrom
    },
    fs::{
        File,
        OpenOptions
    }
};

use super::super::RAM;

/// Save RAM.
/// This may or may not exist based on the cartridge.
pub trait SRAM {
    fn read(&self, addr: u32) -> u8;
    fn write(&mut self, addr: u32, data: u8);

    /// Flush the SRAM to the save file specified.
    fn flush(&mut self);
}

pub fn create_sram(file_name: &str, size: usize) -> Result<Box<dyn SRAM>, String> {
    if size == 0 {
        Ok(Box::new(EmptySRAM::new()))
    } else {
        let sram = SizedSRAM::new(file_name, size)?;
        Ok(Box::new(sram))
    }
}

/// Used in cartridges that have SRAM.
pub struct SizedSRAM {
    save_file:  BufWriter<File>,
    ram:        RAM,

    mask:       u32,  // Mask when reading/writing

    dirty:      bool,
}

impl SizedSRAM {
    fn new(file_name: &str, size: usize) -> Result<Self, String> {
        let mut ram = RAM::new(size);

        if let Ok(file) = File::open(file_name) {
            let mut save_reader = BufReader::new(file);
            save_reader.read_exact(&mut ram.data).map_err(|e| e.to_string())?;
        } else {
            let file = File::create(file_name).map_err(|e| e.to_string())?;
            file.set_len(size as u64).map_err(|e| e.to_string())?;
        }

        let file = OpenOptions::new()
            .write(true)
            .open(file_name)
            .map_err(|e| e.to_string())?;

        Ok(SizedSRAM {
            save_file:  BufWriter::new(file),
            ram:        ram,

            mask:       (size - 1) as u32,

            dirty:      false
        })
    }
}

impl SRAM for SizedSRAM {

    fn read(&self, addr: u32) -> u8 {
        self.ram.read(addr & self.mask)
    }

    fn write(&mut self, addr: u32, data: u8) {
        self.ram.write(addr & self.mask, data);
        self.dirty = true;
    }

    fn flush(&mut self) {
        if self.dirty {
            self.save_file.seek(SeekFrom::Start(0)).expect("Couldn't seek to start of save file!");

            self.save_file.write_all(&self.ram.data).expect("Couldn't write to save file!");

            self.dirty = false;
        }
    }
}

/// Used in cartridges that don't have SRAM.
pub struct EmptySRAM {}

impl EmptySRAM {
    pub fn new() -> Self {
        Self {}
    }
}

impl SRAM for EmptySRAM {
    fn read(&self, _: u32) -> u8 {
        0
    }

    fn write(&mut self, _: u32, _: u8) {}

    fn flush(&mut self) {}
}
