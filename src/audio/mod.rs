// APU (SPC-700)

// CPU-side of APU. Sends and receives to/from audio thread, direct connection with CPU.
pub struct APU {
    ready:  bool,

    port_0: u8,
    port_1: u8,
    port_2: u8,
    port_3: u8
}

impl APU {
    pub fn new() -> Self {
        APU {
            ready:  false,

            port_0: 0xAA,
            port_1: 0xBB,
            port_2: 0,
            port_3: 0,
        }
    }

    pub fn read_port_0(&mut self) -> u8 {
        self.ready = true;
        let ret = self.port_0;
        self.port_0 = 0xAA;
        ret
    }

    pub fn read_port_1(&mut self) -> u8 {
        let ret = self.port_1;
        self.port_1 = 0xBB;
        ret
    }

    pub fn read_port_2(&mut self) -> u8 {
        self.port_2
    }

    pub fn read_port_3(&mut self) -> u8 {
        self.port_3
    }

    pub fn write_port_0(&mut self, data: u8) {
        if self.ready {
            self.port_0 = data;
        }
    }

    pub fn write_port_1(&mut self, data: u8) {
        if self.ready {
            self.port_1 = data;
        }
    }

    pub fn write_port_2(&mut self, data: u8) {
        self.port_2 = data;
    }

    pub fn write_port_3(&mut self, data: u8) {
        self.port_3 = data;
    }
}