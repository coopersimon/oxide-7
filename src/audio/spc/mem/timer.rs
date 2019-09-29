// Timers for SPC-700.

pub struct Timer {
    timer_mod:  u8,     // Timer modulo set by SPC
    counter:    u8,     // Counter

    timer:          u8,     // Internal timer
    period:         usize,  // Number of cycles needed to inc internal timer
    cycle_count:    usize,  // Current cycles since last inc
}

impl Timer {
    pub fn new(period: usize) -> Self {
        Timer {
            timer_mod:  0,
            counter:    0,

            timer:          0,
            period:         period,
            cycle_count:    0,
        }
    }

    pub fn clock(&mut self, cycles: usize) {
        self.cycle_count += cycles;

        if self.cycle_count >= self.period {
            self.cycle_count -= self.period;

            self.timer = self.timer.wrapping_add(1);

            if self.timer == self.timer_mod {
                self.timer = 0;
                self.counter = self.counter.wrapping_add(1) % 0xF;
            }
        }
    }

    pub fn write_timer_modulo(&mut self, data: u8) {
        self.timer_mod = data;
    }

    pub fn read_counter(&mut self) -> u8 {
        let ret = self.counter;
        self.counter = 0;
        ret
    }

    pub fn reset(&mut self) {
        self.counter = 0;
        self.timer = 0;
    }
}