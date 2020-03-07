// PPU
// Owns the video memory and is responsible for communicating with the renderer.

mod ram;
mod render;

use std::sync::{
    Arc,
    Mutex
};

use bitflags::bitflags;

use crate::{
    common::Interrupt,
    constants::{
        timing,
        screen
    },
};

use ram::VideoMem;

type VRamRef = Arc<Mutex<VideoMem>>;

bitflags! {
    #[derive(Default)]
    struct IntEnable: u8 {
        const ENABLE_NMI    = bit!(7);
        const ENABLE_IRQ_Y  = bit!(5);
        const ENABLE_IRQ_X  = bit!(4);
        const AUTO_JOYPAD   = bit!(0);
    }
}

bitflags! {
    #[derive(Default)]
    struct PPUStatus: u8 {
        const V_BLANK = bit!(7);
        const H_BLANK = bit!(6);
    }
}

// Signal from the PPU.
#[derive(PartialEq)]
pub enum PPUSignal {
    None,           // No signal.
    Int(Interrupt), // Interrupt(s) or VBlank triggered:
        //NMI triggered by entering V-blank period.
        //IRQ triggered by X or Y coord.
        //V-Blank period entered without NMI.
    HBlank,         // H-Blank period entered.
    Delay,          // Delay CPU by 40 cycles in middle of scanline.
    FrameStart,     // Frame begin. Reset HDMA.
}

// PPU internal state
#[derive(Debug)]
enum PPUState {
    HBlankLeft,         // The left side of the screen, before drawing begins.
    DrawingBeforePause, // Drawing the line.
    DrawingAfterPause,  // Drawing the line, after the CPU delay.
    HBlankRight,        // The right side of the screen, after drawing ends.
    VBlank              // Vertical blanking period.
}

pub struct PPU {
    state:          PPUState,

    mem:            VRamRef,

    cycle_count:    usize,  // Current cycle count, into the scanline (0-1364)
    scanline:       usize,  // Current scanline

    int_enable:     IntEnable,
    status:         PPUStatus,
    nmi_flag:       u8,     // Top bit set if NMI occurs.
    irq_flag:       u8,     // Top bit set if IRQ occurs.
    h_timer:        u16,    // $4207-8, for triggering IRQ.
    h_cycle:        usize,  // Cycle into line to fire IRQ on.
    v_timer:        u16,    // $4209-a, for triggering IRQ.

    renderer:       render::RenderThread,
    enable_render:  bool,
}

impl PPU {
    pub fn new() -> Self {
        let mem = Arc::new(Mutex::new(VideoMem::new()));

        PPU {
            state:          PPUState::VBlank,
            mem:            mem.clone(),

            cycle_count:    0,
            scanline:       0,

            int_enable:     IntEnable::default(),
            status:         PPUStatus::default(),
            nmi_flag:       0,
            irq_flag:       0,
            h_timer:        0,
            h_cycle:        0,
            v_timer:        0,

            renderer:       render::RenderThread::new(mem),
            enable_render:  true,
        }
    }

    // Enable or disable rendering (from outside).
    pub fn enable_rendering(&mut self, enable: bool) {
        self.enable_render = enable;
    }

    pub fn start_frame(&mut self, frame: Arc<Mutex<[u8]>>) {
        self.renderer.start_frame(frame);
    }

    // Memory access from CPU / B Bus
    pub fn read_mem(&mut self, addr: u8) -> u8 {
        self.mem.lock().unwrap().read(addr)
    }

    pub fn write_mem(&mut self, addr: u8, data: u8) {
        self.mem.lock().unwrap().write(addr, data);
    }

    // Misc
    pub fn get_status(&mut self) -> u8 {
        self.status.bits()
    }

    pub fn latch_hv(&mut self) -> u8 {
        self.mem.lock().unwrap().set_latched_hv(
            (self.cycle_count / timing::DOT_TIME) as u16,   // H
            self.scanline as u16                            // V
        );
        0
    }

    // Timing
    pub fn clock(&mut self, cycles: usize) -> PPUSignal {
        use PPUState::*;
        self.cycle_count += cycles;

        let signal = match self.state {
            VBlank if (self.scanline == 0) && (self.cycle_count >= timing::SCANLINE_OFFSET) => {
                //self.renderer.draw_line(0);

                self.change_state(DrawingBeforePause);
                PPUSignal::FrameStart
            },
            HBlankLeft if self.cycle_count >= timing::SCANLINE_OFFSET => {
                if self.scanline <= screen::V_RES {
                    self.renderer.draw_line((self.scanline - 1) as usize);
                    self.change_state(DrawingBeforePause)
                } else {
                    //self.renderer.frame_end();
                    self.change_state(VBlank)
                }
            },
            DrawingBeforePause if self.cycle_count >= timing::PAUSE_START => {
                self.change_state(DrawingAfterPause)
            },
            DrawingAfterPause if self.cycle_count >= timing::H_BLANK_TIME => {
                // Enter blanking period.
                self.change_state(HBlankRight)
            },
            HBlankRight if self.cycle_count >= timing::SCANLINE => {
                self.change_state(HBlankLeft)
            },
            VBlank if self.cycle_count >= timing::SCANLINE => {
                self.cycle_count -= timing::SCANLINE;
                self.scanline += 1;

                if self.scanline >= screen::NUM_SCANLINES {
                    self.scanline -= screen::NUM_SCANLINES;
                }

                if self.int_enable.contains(IntEnable::ENABLE_IRQ_Y) && (self.scanline == (self.v_timer as usize)) {
                    self.trigger_interrupt(Interrupt::IRQ)
                } else {
                    PPUSignal::None
                }
            },
            _ => PPUSignal::None
        };

        if signal == PPUSignal::None {
            if self.int_enable.contains(IntEnable::ENABLE_IRQ_X) && (self.cycle_count >= self.h_cycle) {
                self.trigger_interrupt(Interrupt::IRQ)
            } else {
                PPUSignal::None
            }
        } else {
            signal
        }
    }

    // Interrupts
    pub fn set_int_enable(&mut self, data: u8) {
        self.int_enable = IntEnable::from_bits_truncate(data);
    }

    pub fn set_h_timer_lo(&mut self, data: u8) {
        self.h_timer = set_lo!(self.h_timer, data);
        self.h_cycle = (self.h_timer as usize) * timing::DOT_TIME;
    }

    pub fn set_h_timer_hi(&mut self, data: u8) {
        self.h_timer = set_hi!(self.h_timer, data);
        self.h_cycle = (self.h_timer as usize) * timing::DOT_TIME;
    }

    pub fn set_v_timer_lo(&mut self, data: u8) {
        self.v_timer = set_lo!(self.v_timer, data);
    }

    pub fn set_v_timer_hi(&mut self, data: u8) {
        self.v_timer = set_hi!(self.v_timer, data);
    }

    pub fn get_nmi_flag(&mut self) -> u8 {
        let ret = self.nmi_flag;
        self.nmi_flag = 0;
        ret
    }

    pub fn get_irq_flag(&mut self) -> u8 {
        let ret = self.irq_flag;
        self.irq_flag = 0;
        ret
    }
}

// Internal
impl PPU {
    // TODO: change this to transition?
    fn change_state(&mut self, state: PPUState) -> PPUSignal {
        self.state = state;
        match self.state {
            PPUState::DrawingBeforePause => {
                self.nmi_flag = 0;
                self.irq_flag = 0;
                self.toggle_vblank(false);
                self.toggle_hblank(false);

                PPUSignal::None
            },
            PPUState::DrawingAfterPause => {
                PPUSignal::Delay
            },
            PPUState::VBlank => {
                self.toggle_vblank(true);
                self.toggle_hblank(false);

                {
                    let mut mem = self.mem.lock().unwrap();
                    if mem.get_bg_registers().in_fblank() {
                        mem.oam_reset();
                    }
                }

                self.trigger_interrupt(Interrupt::NMI)
            },
            PPUState::HBlankRight => {
                self.toggle_hblank(true);
                PPUSignal::HBlank
            },
            PPUState::HBlankLeft => self.inc_scanline()
        }
    }

    // Increment the scanline and return an IRQ interrupt if necessary.
    fn inc_scanline(&mut self) -> PPUSignal {
        self.cycle_count -= timing::SCANLINE;
        self.scanline += 1;

        if self.int_enable.contains(IntEnable::ENABLE_IRQ_Y) && (self.scanline == (self.v_timer as usize)) {
            self.trigger_interrupt(Interrupt::IRQ)
        } else {
            PPUSignal::None
        }
    }

    // Set the appropriate bit and return the appropriate signal.
    // TODO: allow triggering of multiple interrupts
    // TODO: handle this better in general
    fn trigger_interrupt(&mut self, int: Interrupt) -> PPUSignal {
        if int.contains(Interrupt::NMI) {
            self.nmi_flag |= bit!(7);
            if self.int_enable.contains(IntEnable::ENABLE_NMI) {
                PPUSignal::Int(Interrupt::NMI)
            } else {
                PPUSignal::Int(Interrupt::VBLANK)
            }
        } else if int.contains(Interrupt::IRQ) {
            self.irq_flag |= bit!(7);
            PPUSignal::Int(Interrupt::IRQ)
        } else {
            PPUSignal::None
        }
    }

    // Toggle blanking modes.
    fn toggle_vblank(&mut self, vblank: bool) {
        self.status.set(PPUStatus::V_BLANK, vblank);
    }

    fn toggle_hblank(&mut self, hblank: bool) {
        self.status.set(PPUStatus::H_BLANK, hblank);
    }
}
