// PPU
// Owns the video memory and is responsible for communicating with the renderer.

mod ram;
mod render;

mod vulkan;

use std::{
    sync::{
        Arc,
        Mutex,
        mpsc::{
            channel,
            Sender,
            Receiver
        }
    },
    thread
};

use winit::EventsLoop;

use bitflags::bitflags;

use crate::{
    common::Interrupt,
    constants::{
        timing,
        screen
    },
    joypad::JoypadMem
};

use ram::VideoMem;
use render::*;

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
pub enum PPUSignal {
    None,       // No signal
    NMI,        // NMI triggered by entering V-blank period.
    IRQ,        // IRQ triggered by X or Y coord.
    HBlank,     // H-Blank period entered
    // Delay?   // Delay CPU by 40 cycles in middle of scanline
}

pub struct PPU {
    mem:            VRamRef,

    joypads:        JoypadMem,

    cycle_count:    usize,  // Current cycle count, into the scanline (0-1364)
    scanline:       usize,  // Current scanline

    int_enable:     IntEnable,
    status:         PPUStatus,
    nmi_flag:       u8,     // Top bit set if NMI occurs.
    irq_flag:       u8,     // Top bit set if IRQ occurs.
    h_timer:        u16,    // $4207-8, for triggering IRQ.
    h_cycle:        usize,  // Cycle into line to fire IRQ on.
    v_timer:        u16,    // $4209-a, for triggering IRQ.

    command_tx:     Sender<VideoCommand>,
    signal_rx:      Receiver<VideoSignal>
}

impl PPU {
    pub fn new() -> Self {
        let mem = Arc::new(Mutex::new(VideoMem::new()));
        let thread_mem = mem.clone();

        let (command_tx, command_rx) = channel();
        let (signal_tx, signal_rx) = channel();

        thread::spawn(move || {
            use VideoCommand::*;

            let events_loop = EventsLoop::new();
            let mut renderer = vulkan::Renderer::new(thread_mem, &events_loop);

            // Process commands.
            while let Ok(command) = command_rx.recv() {
                let signal = match command {
                    FrameStart => {
                        renderer.frame_start();
                        VideoSignal::None
                    },
                    DrawLine(y) => {
                        renderer.draw_line(y);
                        VideoSignal::HBlank
                    },
                    FrameEnd => {
                        renderer.frame_end();
                        // Read events loop.
                        let joypad_mem = crate::joypad::Button::default();
                        VideoSignal::VBlank(joypad_mem)
                    },
                    None => VideoSignal::None
                };

                signal_tx.send(signal).expect("Could not send signal from video thread.");
            }
        });

        PPU {
            mem:            mem,
            joypads:        JoypadMem::new(),

            cycle_count:    0,
            scanline:       0,

            int_enable:     IntEnable::default(),
            status:         PPUStatus::default(),
            nmi_flag:       0,
            irq_flag:       0,
            h_timer:        0,
            h_cycle:        0,
            v_timer:        0,

            command_tx:     command_tx,
            signal_rx:      signal_rx
        }
    }

    // Memory access from CPU / B Bus
    pub fn read_mem(&mut self, addr: u8) -> u8 {
        if let Ok(mut mem) = self.mem.try_lock() {
            mem.read(addr)
        } else {
            0
        }
    }

    pub fn write_mem(&mut self, addr: u8, data: u8) {
        if let Ok(mut mem) = self.mem.try_lock() {
            mem.write(addr, data);
        }
    }

    // Misc
    pub fn get_status(&mut self) -> u8 {
        self.joypads.is_ready()
    }

    // Joypad access
    pub fn read_joypad(&mut self, addr: u16) -> u8 {
        self.joypads.read(addr)
    }

    pub fn joypad_latch(&mut self) {
        self.joypads.latch_all()
    }

    // Timing
    pub fn clock(&mut self, cycles: usize) -> PPUSignal {
        self.cycle_count += cycles;

        if self.cycle_count >= timing::SCANLINE {

            self.cycle_count -= timing::SCANLINE;
            self.scanline += 1;

            if self.scanline >= screen::NUM_SCANLINES {
                self.scanline -= screen::NUM_SCANLINES;
            }

            if self.int_enable.contains(IntEnable::ENABLE_IRQ_Y) && (self.scanline == (self.v_timer as usize)) {
                return self.trigger_interrupt(Interrupt::IRQ);
            }

        } else if self.cycle_count >= timing::H_BLANK_TIME {
            // Enter blanking period.
            let wait_for_blank = self.signal_rx.recv().unwrap();

            match wait_for_blank {
                VideoSignal::VBlank(j) => {
                    self.toggle_vblank(true);
                    self.joypads.set_buttons(j, 0);

                    if self.int_enable.contains(IntEnable::AUTO_JOYPAD) {
                        self.joypads.prepare_read();
                    }

                    if self.int_enable.contains(IntEnable::ENABLE_NMI) {
                        return self.trigger_interrupt(Interrupt::NMI);
                    }
                },
                VideoSignal::HBlank => {
                    self.toggle_hblank(true);
                    return PPUSignal::HBlank;
                },
                VideoSignal::None   => {}
            }

        } else if self.cycle_count >= timing::SCANLINE_OFFSET {

            if self.scanline == 1 {
                self.nmi_flag = 0;
                self.toggle_vblank(false);
                self.command_tx.send(VideoCommand::FrameStart).unwrap();
                self.command_tx.send(VideoCommand::DrawLine(self.scanline as u8)).unwrap();

            } else if self.scanline <= 224 {
                self.toggle_hblank(false);
                self.command_tx.send(VideoCommand::DrawLine(self.scanline as u8)).unwrap();

            } else if self.scanline == 225 {
                self.toggle_hblank(false);
                self.command_tx.send(VideoCommand::FrameEnd).unwrap();
                
            } else {
                self.command_tx.send(VideoCommand::None).unwrap();
            }
        }

        if self.int_enable.contains(IntEnable::ENABLE_IRQ_X) && (self.cycle_count >= self.h_cycle) {
            self.trigger_interrupt(Interrupt::IRQ)
        } else {
            PPUSignal::None
        }
    }

    // Interrupts
    pub fn set_int_enable(&mut self, data: u8) {
        self.int_enable = IntEnable::from_bits_truncate(data);
        self.joypads.enable_counter(data);
    }

    pub fn set_h_timer_lo(&mut self, data: u8) {
        self.h_timer = set_lo!(self.h_timer, data);
        self.h_cycle = (self.h_timer as usize) * 4;
    }

    pub fn set_h_timer_hi(&mut self, data: u8) {
        self.h_timer = set_hi!(self.h_timer, data);
        self.h_cycle = (self.h_timer as usize) * 4;
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
    // Set the appropriate bit and return the appropriate signal.
    fn trigger_interrupt(&mut self, int: Interrupt) -> PPUSignal {
        match int {
            Interrupt::NMI => {
                self.nmi_flag |= bit!(7);
                PPUSignal::NMI
            },
            Interrupt::IRQ => {
                self.irq_flag |= bit!(7);
                PPUSignal::IRQ
            },
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