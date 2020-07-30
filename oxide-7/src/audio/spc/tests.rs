// Run blargg's test files

use std::{
    io::{
        BufReader,
        Read,
    },
    fs::File
};

use super::super::mem::SPCMem;

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

pub struct TestSPCMem {
    ram:    Vec<u8>
}

impl TestSPCMem {
    fn new(file_name: &str) -> Self {
        let prog_file = File::open(file_name).expect(&format!("Couldn't open file {}", file_name));
        let mut reader = BufReader::new(prog_file);

        let mut buffer = vec![0; 1024 * 64];

        reader.read(&mut buffer[0x400..]).unwrap();

        Self {
            ram:    buffer
        }
    }
}

impl SPCMem for TestSPCMem {

    fn read(&mut self, addr: u16) -> u8 {
        if addr >= 0xFFC0 {
            IPL_ROM[(addr - 0xFFC0) as usize]
        } else {
            self.ram[addr as usize]
        }
    }
    fn write(&mut self, addr: u16, data: u8) {
        self.ram[addr as usize] = data;
    }
    fn clock(&mut self, _cycles: usize) {}
}

fn run_test(name: &str) {
    println!("Running for {}", name);

    let mem = TestSPCMem::new(name);
    let mut spc = super::SPC::new(mem);
    spc.set_pc(0x430);

    while spc.read_mem(0x8001) != 0xDE && spc.read_mem(0x8002) != 0xB0 && spc.read_mem(0x8003) != 0x61 {
        spc.step();
    }

    while spc.read_mem(0x8000) == 0x80 {
        spc.step();
    }

    let mut str_buf = Vec::new();
    let mut i = 0;
    loop {
        let c = spc.read_mem(0x8004 + i);
        str_buf.push(c);
        i += 1;
        if c == 0 {
            break;
        }
    }
    let s = String::from_utf8(str_buf).unwrap();
    println!("{}", s);
    assert_eq!(0x00, spc.read_mem(0x8000));
}

#[test]
fn run_all_tests() {
    let test_names = vec![
        "./test/spc/tests/CPU Instructions_Edge arith",
        "./test/spc/tests/CPU Instructions_Full DAA DAS",
        "./test/spc/tests/CPU Instructions_Full CMP",
        "./test/spc/tests/CPU_addw and subw",
        "./test/spc/tests/CPU_wrap-around mem",
    ];

    for name in test_names.iter() {
        run_test(name);
    }
}