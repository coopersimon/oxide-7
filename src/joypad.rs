// Joypad registers

use bitflags::bitflags;

bitflags! {
    // Flags for buttons.
    #[derive(Default)]
    pub struct Button: u16 {
        const B      = bit!(15, u16);
        const Y      = bit!(14, u16);
        const SELECT = bit!(13, u16);
        const START  = bit!(12, u16);
        const UP     = bit!(11, u16);
        const DOWN   = bit!(10, u16);
        const LEFT   = bit!(9, u16);
        const RIGHT  = bit!(8, u16);
        const A      = bit!(7, u16);
        const X      = bit!(6, u16);
        const L      = bit!(5, u16);
        const R      = bit!(4, u16);
    }
}

// The Joypads
pub struct JoypadMem {
    joypads: [Joypad; 4],   // "External" joypads.

    joypad_regs: [u8; 8],   // Regs 4218-421F

    counter:    bool,       // Reg 4200 bit 0
    ready:      bool,       // Reg 4212 bit 0
}

impl JoypadMem {
    pub fn new() -> Self {
        JoypadMem {
            joypads:        [Joypad::new(); 4],

            joypad_regs:    [0; 8],

            counter:        false,
            ready:          false,
        }
    }

    // Set buttons externally.
    pub fn set_buttons(&mut self, button: Button, joypad: usize) {
        self.joypads[joypad].set_buttons(button);
    }

    // Set new-style joypad reading.
    pub fn enable_counter(&mut self, val: u8) {
        self.counter = test_bit!(val, 0, u8);
    }

    // Poll if the joypad is ready to be read from (new-style).
    pub fn is_ready(&self) -> u8 {
        if self.ready {0} else {1}
    }

    // Called at V-blank.
    pub fn prepare_read(&mut self) {
        if self.counter {
            self.ready = false;

            // TODO: clock the following.
            for (i, j) in self.joypads.iter_mut().enumerate() {
                j.latch();
                let regs = j.read();
                let reg_base = i * 2;
                self.joypad_regs[reg_base] = regs.0;
                self.joypad_regs[reg_base + 1] = regs.1;
            }

            self.ready = true;
        }
    }

    // Call to latch all joypads.
    pub fn latch_all(&mut self) {
        for j in self.joypads.iter_mut() {
            j.latch();
        }
    }

    // Read joypad register.
    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x4016 => self.joypads[0].shift_bit(),  // Shift the next one too?
            0x4017 => self.joypads[1].shift_bit(),

            0x4218 => self.joypad_regs[0],
            0x4219 => self.joypad_regs[1],
            0x421A => self.joypad_regs[2],
            0x421B => self.joypad_regs[3],
            0x421C => self.joypad_regs[4],
            0x421D => self.joypad_regs[5],
            0x421E => self.joypad_regs[6],
            0x421F => self.joypad_regs[7],

            _ => 0,
        }
    }
}

// A single joypad
#[derive(Clone, Copy)]
struct Joypad {
    // External presses
    buttons:    Button,

    // Internal latched data
    register:   u16,
}

impl Joypad {
    fn new() -> Self {
        Joypad {
            buttons:    Button::default(),
            register:   0xFFFF,
        }
    }

    // Set buttons externally.
    fn set_buttons(&mut self, button: Button) {
        self.buttons.insert(button);
    }

    // Latch data into internal register.
    // Important!: buttons must be set for the previous frame before this is done, or there will be a delay.
    fn latch(&mut self) {
        self.register = self.buttons.bits() as u16;
        self.buttons = Button::default();
    }

    // Shift a single bit out (old-style read).
    fn shift_bit(&mut self) -> u8 {
        let bit_set = test_bit!(self.register, 15);
        self.register = (self.register << 1) | 1;
        if bit_set {1} else {0}
    }

    // Read the contents of the register (new-style read).
    fn read(&mut self) -> (u8, u8) {
        let (lo, hi) = (lo!(self.register), hi!(self.register));
        self.register = 0xFFFF;
        (lo, hi)
    }
}