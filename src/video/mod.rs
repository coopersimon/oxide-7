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

impl IntEnable {
    fn all_irq() -> IntEnable {
        IntEnable::ENABLE_IRQ_X | IntEnable::ENABLE_IRQ_Y
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
#[derive(Debug, PartialEq)]
enum PPUState {
    HBlankLeft,         // The left side of the screen, before drawing begins.
    DrawingBeforePause, // Drawing the line.
    DrawingAfterPause,  // Drawing the line, after the CPU delay.
    HBlankRight,        // The right side of the screen, after drawing ends.
    VBlank              // Vertical blanking period.
}

// Background (for use by child modules)
#[derive(Clone, Copy)]
pub enum BG {
    _1,
    _2,
    _3,
    _4
}

impl BG {
    fn all() -> &'static [BG; 4] {
        const BGS: [BG; 4] = [BG::_1, BG::_2, BG::_3, BG::_4];
        &BGS
    }
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
    h_irq_latch:    bool,   // Latched if the horizontal IRQ is triggered.

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
            h_irq_latch:    false,

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

        let transition = match self.state {
            VBlank              if self.scanline == 0                       => Some(PPUTransition::ExitVBlank),
            DrawingBeforePause  if self.cycle_count >= timing::PAUSE_START  => Some(PPUTransition::CPUPause),
            DrawingAfterPause   if self.cycle_count >= timing::H_BLANK_TIME => Some(PPUTransition::EnterHBlank),
            HBlankRight         if self.cycle_count >= timing::SCANLINE     => Some(PPUTransition::NextLine),
            HBlankLeft          if self.scanline > screen::V_RES            => Some(PPUTransition::EnterVBlank),
            HBlankLeft          if (self.cycle_count >= timing::SCANLINE_OFFSET)
                                && (self.scanline <= screen::V_RES)         => Some(PPUTransition::ExitHBlank),
            VBlank              if self.cycle_count >= timing::SCANLINE     => Some(PPUTransition::NextLine),
            _ => None
        };

        let signal = if let Some(transition) = transition {
            self.transition_state(transition)
        } else {
            PPUSignal::None
        };

        if signal == PPUSignal::None {
            if self.check_x_irq() {
                self.h_irq_latch = true;
                self.trigger_irq()
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
        self.h_irq_latch = false;
    }

    pub fn set_h_timer_hi(&mut self, data: u8) {
        self.h_timer = set_hi!(self.h_timer, data);
        self.h_cycle = (self.h_timer as usize) * timing::DOT_TIME;
        self.h_irq_latch = false;
    }

    pub fn set_v_timer_lo(&mut self, data: u8) {
        self.v_timer = set_lo!(self.v_timer, data);
    }

    pub fn set_v_timer_hi(&mut self, data: u8) {
        self.v_timer = set_hi!(self.v_timer, data);
    }

    pub fn get_nmi_flag(&mut self) -> u8 {
        std::mem::replace(&mut self.nmi_flag, 0)
    }

    pub fn get_irq_flag(&mut self) -> u8 {
        std::mem::replace(&mut self.irq_flag, 0)
    }
}

// Each transition has a source and target state associated with it.
// When transitioning, a signal can be emitted.
enum PPUTransition {
    ExitVBlank,
    CPUPause,
    EnterHBlank,
    NextLine,
    ExitHBlank,
    EnterVBlank,
}

// Internal
impl PPU {
    // Transition to the appropriate state and emit a relevant signal.
    fn transition_state(&mut self, transition: PPUTransition) -> PPUSignal {
        use PPUTransition::*;
        match transition {
            ExitVBlank => {
                self.nmi_flag = 0;
                self.irq_flag = 0;
                self.toggle_vblank(false);
                self.toggle_hblank(false);
                self.state = PPUState::DrawingBeforePause;
                PPUSignal::FrameStart
            },
            CPUPause => {
                self.state = PPUState::DrawingAfterPause;
                PPUSignal::Delay
            },
            EnterHBlank => {
                self.toggle_hblank(true);
                self.state = PPUState::HBlankRight;
                PPUSignal::HBlank
            },
            NextLine => {
                self.cycle_count -= timing::SCANLINE;
                self.h_irq_latch = false;
                self.scanline += 1;

                if self.scanline >= screen::NUM_SCANLINES {
                    self.scanline -= screen::NUM_SCANLINES;
                }

                if self.state == PPUState::HBlankRight {
                    self.state = PPUState::HBlankLeft;
                }

                if self.check_y_irq() {
                    self.trigger_irq()
                } else {
                    PPUSignal::None
                }
            },
            ExitHBlank => {
                self.renderer.draw_line((self.scanline - 1) as usize);
                self.state = PPUState::DrawingBeforePause;
                PPUSignal::None
            },
            EnterVBlank => {
                //self.renderer.frame_end();
                self.toggle_vblank(true);
                self.toggle_hblank(false);

                {
                    let mut mem = self.mem.lock().unwrap();
                    if mem.get_bg_registers().in_fblank() {
                        mem.oam_reset();
                    }
                }

                self.state = PPUState::VBlank;
                self.trigger_nmi()
            },
        }
    }

    // See if IRQ should be triggered.
    // The Y IRQ check should happen at the start of each line.
    fn check_y_irq(&self) -> bool {
        let enabled = (self.int_enable & IntEnable::all_irq()) == IntEnable::ENABLE_IRQ_Y;
        enabled && (self.scanline == (self.v_timer as usize))
    }

    // The X IRQ check should happen anytime the current X increases (every 4 cycles).
    // If the Y IRQ line is enabled too, it will only trigger on the correct line.
    // Otherwise it will trigger on every line.
    fn check_x_irq(&self) -> bool {
        let irq = self.int_enable & IntEnable::all_irq();

        if !self.h_irq_latch {
            if irq == IntEnable::all_irq() {
                (self.scanline == (self.v_timer as usize)) && (self.cycle_count >= self.h_cycle)
            } else if irq == IntEnable::ENABLE_IRQ_X {
                self.cycle_count >= self.h_cycle
            } else {
                false
            }
        } else {
            false
        }
    }

    // Trigger interrupts.
    fn trigger_nmi(&mut self) -> PPUSignal {
        self.nmi_flag |= bit!(7);
        if self.int_enable.contains(IntEnable::ENABLE_NMI) {
            PPUSignal::Int(Interrupt::NMI)
        } else {
            PPUSignal::Int(Interrupt::VBLANK)
        }
    }

    fn trigger_irq(&mut self) -> PPUSignal {
        self.irq_flag |= bit!(7);
        PPUSignal::Int(Interrupt::IRQ)
    }

    // Toggle blanking modes.
    fn toggle_vblank(&mut self, vblank: bool) {
        self.status.set(PPUStatus::V_BLANK, vblank);
    }

    fn toggle_hblank(&mut self, hblank: bool) {
        self.status.set(PPUStatus::H_BLANK, hblank);
    }
}
