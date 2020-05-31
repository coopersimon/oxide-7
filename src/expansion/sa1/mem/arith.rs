// Arithmetic unit

enum ArithMode {
    Multiply,
    Divide,
    MultiplySum,
    Res
}

impl From<u8> for ArithMode {
    fn from(val: u8) -> Self {
        use ArithMode::*;
        match val & 0x3 {
            0 => Multiply,
            1 => Divide,
            2 => MultiplySum,
            3 => Res,
            _ => unreachable!()
        }
    }
}


pub struct Arithmetic {
    mode:       ArithMode,

    param_a:    u16,
    param_b:    u16,

    sum:        u64,
    overflow:   bool,
}

impl Arithmetic {
    pub fn new() -> Self {
        Self {
            mode:       ArithMode::Multiply,
            param_a:    0,
            param_b:    0,
            sum:        0,
            overflow:   false,
        }
    }

    pub fn write_control(&mut self, data: u8) {
        self.mode = data.into();
        if test_bit!(data, 1, u8) {
            self.sum = 0;
        }
        self.overflow = false;
    }

    pub fn write_param_a_lo(&mut self, data: u8) {
        self.param_a = set_lo!(self.param_a, data);
    }

    pub fn write_param_a_hi(&mut self, data: u8) {
        self.param_a = set_hi!(self.param_a, data);
    }

    pub fn write_param_b_lo(&mut self, data: u8) {
        self.param_b = set_lo!(self.param_b, data);
    }

    pub fn write_param_b_hi(&mut self, data: u8) {
        self.param_b = set_hi!(self.param_b, data);
        self.do_op();
    }

    pub fn read_result_0(&self) -> u8 {
        self.sum as u8
    }

    pub fn read_result_1(&self) -> u8 {
        (self.sum >> 8) as u8
    }

    pub fn read_result_2(&self) -> u8 {
        (self.sum >> 16) as u8
    }

    pub fn read_result_3(&self) -> u8 {
        (self.sum >> 24) as u8
    }

    pub fn read_result_4(&self) -> u8 {
        (self.sum >> 32) as u8
    }

    pub fn read_ovf(&self) -> u8 {
        if self.overflow {
            bit!(7_u8)
        } else {
            0
        }
    }
}

impl Arithmetic {
    fn do_op(&mut self) {
        use ArithMode::*;
        self.sum = match self.mode {
            Multiply => {
                let signed_a = (self.param_a as i16) as i32;
                let signed_b = (self.param_b as i16) as i32;
                let result = signed_a * signed_b;
                (result as u32) as u64
            },
            Divide => {
                let signed_a = (self.param_a as i16) as i32;
                let signed_b = self.param_b as i32;
                if signed_b == 0 {
                    0
                } else {
                    let result = ((signed_a / signed_b) as u32) as u16;
                    let remainder = ((signed_a % signed_b) as u32) as u16;
                    ((remainder as u64) << 16) | (result as u64)
                }
            },
            MultiplySum | Res => {
                const MASK_40_BIT: u64 = 0xFF_FFFF_FFFF;
                let signed_a = (self.param_a as i16) as i32;
                let signed_b = (self.param_b as i16) as i32;
                let result = signed_a * signed_b;

                let result_sign = test_bit!(result as u32, 31, u32);
                let result40 = ((result as i64) as u64) & MASK_40_BIT;
                let add_mult = (self.sum + result40) & MASK_40_BIT;
                self.overflow = (self.sum > add_mult && !result_sign) || (self.sum < add_mult && result_sign);
                add_mult
            },
        };
    }
}