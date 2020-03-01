// Memory
mod bus;
mod dma;
mod rom;

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

pub use bus::MemBus;

// Random access memory.
pub struct RAM {
    data: Vec<u8>
}

impl RAM {
    pub fn new(size: usize) -> Self {
        RAM {
            data: vec![0; size]
        }
    }

    pub fn read(&self, addr: u32) -> u8 {
        self.data[addr as usize]
    }

    pub fn write(&mut self, addr: u32, data: u8) {
        self.data[addr as usize] = data;
    }
}

// Save RAM.
pub struct SRAM {
    save_file:  BufWriter<File>,
    ram:        RAM,

    dirty:      bool,
}

impl SRAM {
    pub fn new(file_name: &str, size: usize) -> Result<Self, String> {
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

        Ok(SRAM {
            save_file:  BufWriter::new(file),
            ram:        ram,

            dirty:      false
        })
    }

    pub fn read(&self, addr: u32) -> u8 {
        self.ram.read(addr)
    }

    pub fn write(&mut self, addr: u32, data: u8) {
        self.ram.write(addr, data);
        self.dirty = true;
    }

    pub fn flush(&mut self) {
        if self.dirty {
            self.save_file.seek(SeekFrom::Start(0)).expect("Couldn't seek to start of save file!");

            self.save_file.write_all(&self.ram.data).expect("Couldn't write to save file!");

            self.dirty = false;
        }
    }
}