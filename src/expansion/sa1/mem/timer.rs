// SA1 Timer

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    struct TimerControl: u8 {
        const TIMER_MODE = bit!(7); // 1 = linear timer
        const VEN = bit!(1);        // V-Enable
        const HEN = bit!(0);        // H-Enable
    }
}

pub struct Timer {
    timer_control:  TimerControl,

    h_count:    u16,    // Dot count
    v_count:    u16,    // Line count
    h_cmp:      u16,    // H compare
    v_cmp:      u16,    // V compare
    latched_h:  u16,    // Latched dot count
    latched_v:  u16,    // Latched line count

    cycle_count: usize,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            timer_control: TimerControl::default(),

            h_count:    0,
            v_count:    0,
            h_cmp:      0,
            v_cmp:      0,
            latched_h:  0,
            latched_v:  0,

            cycle_count: 0,
        }
    }

    // Returns true if interrupt was triggered.
    pub fn clock(&mut self, cycles: usize) -> bool {
        self.cycle_count += cycles;
        let to_inc = (self.cycle_count >> 2) as u16;
        self.cycle_count %= 4;

        self.h_count += to_inc;
        if self.timer_control.contains(TimerControl::TIMER_MODE) {
            if self.h_count > 511 {
                self.h_count -= 511;
                self.v_count += 1;
                if self.v_count > 511 {
                    self.v_count -= 511;
                }
            }
        } else {
            if self.h_count > 340 {
                self.h_count -= 340;
                self.v_count += 1;
                if self.v_count > 223 {
                    self.v_count -= 223;
                }
            }
        }

        match (self.timer_control.contains(TimerControl::HEN), self.timer_control.contains(TimerControl::VEN)) {
            (true, false) if self.h_count == self.h_cmp => true,
            (false, true) if self.v_count == self.v_cmp => true,
            (true, true) if self.h_count == self.h_cmp && self.v_count == self.v_cmp => true,

            _ => false
        }
    }
}

// Memory interface
impl Timer {
    pub fn write_control(&mut self, data: u8) {
        self.timer_control = TimerControl::from_bits_truncate(data);
    }

    pub fn restart(&mut self) {
        self.h_count = 0;
        self.v_count = 0;
    }


    pub fn write_h_lo(&mut self, data: u8) {
        self.h_cmp = set_lo!(self.h_cmp, data);
    }

    pub fn write_h_hi(&mut self, data: u8) {
        self.h_cmp = set_hi!(self.h_cmp, data & 1);
    }

    pub fn write_v_lo(&mut self, data: u8) {
        self.v_cmp = set_lo!(self.v_cmp, data);
    }

    pub fn write_v_hi(&mut self, data: u8) {
        self.v_cmp = set_hi!(self.v_cmp, data & 1);
    }

    pub fn read_h_lo(&mut self) -> u8 {
        self.latched_h = self.h_count;
        self.latched_v = self.v_count;
        lo!(self.latched_h)
    }

    pub fn read_h_hi(&self) -> u8 {
        hi!(self.latched_h)
    }

    pub fn read_v_lo(&self) -> u8 {
        lo!(self.latched_v)
    }

    pub fn read_v_hi(&self) -> u8 {
        hi!(self.latched_v)
    }
}