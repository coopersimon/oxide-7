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

use crate::constants::{
    timing,
    screen
};

use ram::VideoMem;
use render::*;

type VRamRef = Arc<Mutex<VideoMem>>;

pub struct PPU {
    mem:            VRamRef,

    cycle_count:    usize,
    scanline:       usize,

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
                    DrawLine => {
                        renderer.draw_line();
                        VideoSignal::HBlank
                    },
                    FrameEnd => {
                        renderer.frame_end();
                        VideoSignal::VBlank
                    },
                    None => VideoSignal::None
                };

                signal_tx.send(signal).expect("Could not send signal from video thread.");
            }
        });

        PPU {
            mem:            mem,

            cycle_count:    0,
            scanline:       0,

            command_tx:     command_tx,
            signal_rx:      signal_rx
        }
    }

    // Memory access from CPU -> B Bus
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

    // Timing
    pub fn clock(&mut self, cycles: usize) {
        self.cycle_count += cycles;

        if self.cycle_count >= timing::SCANLINE {

            self.cycle_count -= timing::SCANLINE;
            self.scanline += 1;

            if self.scanline >= screen::NUM_SCANLINES {
                self.scanline -= screen::NUM_SCANLINES;
            }

        } else if self.cycle_count >= timing::H_BLANK_TIME {
            // Enter blanking period.
            let wait_for_blank = self.signal_rx.recv().unwrap();

            // TODO: trigger interrupts
            match wait_for_blank {
                VideoSignal::HBlank => {},
                VideoSignal::VBlank => {},
                VideoSignal::None   => {}
            }

        } else if self.cycle_count >= timing::SCANLINE_OFFSET {

            if self.scanline == 0 {
                self.command_tx.send(VideoCommand::FrameStart).unwrap();
            } else if self.scanline <= 224 {
                self.command_tx.send(VideoCommand::DrawLine).unwrap();
            } else if self.scanline == 225 {
                self.command_tx.send(VideoCommand::FrameEnd).unwrap();
            } else {
                self.command_tx.send(VideoCommand::None).unwrap();
            }
        }
    }
}