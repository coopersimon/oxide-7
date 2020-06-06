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

        let mut h_tick = false;
        let mut v_tick = false;
        self.h_count += to_inc;
        if self.timer_control.contains(TimerControl::TIMER_MODE) {
            if self.h_count >= 512 {
                h_tick = true;
                self.h_count -= 512;
                self.v_count += 1;
                if self.v_count >= 512 {
                    v_tick = true;
                    self.v_count -= 512;
                }
            }
        } else {
            if self.h_count >= 341 {
                h_tick = true;
                self.h_count -= 341;
                self.v_count += 1;
                if self.v_count >= 262 {
                    v_tick = true;
                    self.v_count -= 262;
                }
            }
        }

        match (self.timer_control.contains(TimerControl::HEN), self.timer_control.contains(TimerControl::VEN)) {
            (true, false) => h_tick,
            (false, true) => v_tick,
            (true, true) => h_tick && v_tick,

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
        self.h_count = set_lo!(self.h_count, data);
    }

    pub fn write_h_hi(&mut self, data: u8) {
        self.h_count = set_hi!(self.h_count, data & 1);
    }

    pub fn write_v_lo(&mut self, data: u8) {
        self.v_count = set_lo!(self.v_count, data);
    }

    pub fn write_v_hi(&mut self, data: u8) {
        self.v_count = set_hi!(self.v_count, data & 1);
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