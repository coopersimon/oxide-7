// Tools for communicating with the renderer (and its thread on certain platforms).

mod bgcache;
mod patternmem;
mod drawing;
mod palette;

use std::sync::{
    Arc, Mutex
};

use crossbeam_channel::{
    bounded,
    Sender,
    Receiver
};

// Renderer trait.
pub trait Renderable {
    fn frame_start(&mut self);
    fn draw_line(&mut self, y: u16);
    fn frame_end(&mut self);
}

// Mode
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VideoMode {
    _0 = 0,
    _1 = 1,
    _2 = 2,
    _3 = 3,
    _4 = 4,
    _5 = 5,
    _6 = 6,
    _7 = 7
}

impl From<u8> for VideoMode {
    fn from(val: u8) -> Self {
        match val & bits![2, 1, 0] {
            0 => VideoMode::_0,
            1 => VideoMode::_1,
            2 => VideoMode::_2,
            3 => VideoMode::_3,
            4 => VideoMode::_4,
            5 => VideoMode::_5,
            6 => VideoMode::_6,
            7 => VideoMode::_7,
            _ => unreachable!()
        }
    }
}

pub type RenderTarget = Arc<Mutex<[u8]>>;

#[derive(Clone, Copy, Debug)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

impl Colour {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Colour {
            r: r,
            g: g,
            b: b
        }
    }

    pub const fn zero() -> Colour {
        Colour {
            r: 0,
            g: 0,
            b: 0
        }
    }

    /*const fn red() -> Colour {
        Colour {
            r: 255,
            g: 0,
            b: 0
        }
    }
    const fn blue() -> Colour {
        Colour {
            r: 0,
            g: 0,
            b: 255
        }
    }
    const fn green() -> Colour {
        Colour {
            r: 0,
            g: 255,
            b: 0
        }
    }
    const fn yellow() -> Colour {
        Colour {
            r: 255,
            g: 255,
            b: 0
        }
    }*/
}

// Messages to send to the render thread.
enum RendererMessage {
    StartFrame(RenderTarget),   // Begin frame, and target the provided byte array.
    DrawLine(usize),
}

// Renderer for video that spawns a thread to render on.
pub struct RenderThread {
    sender:     Sender<RendererMessage>,
    receiver:   Receiver<()>,
}

impl RenderThread {
    pub fn new(mem: super::VRamRef) -> Self {
        let (send_msg, recv_msg) = bounded(1);
        let (send_reply, recv_reply) = bounded(1);

        std::thread::spawn(move || {
            use RendererMessage::*;
            let mut target = None;
            let mut renderer = drawing::Renderer::new();

            while let Ok(msg) = recv_msg.recv() {
                match msg {
                    StartFrame(data) => {
                        target = Some(data);
                    },
                    DrawLine(y) => {
                        let mut mem = mem.lock().unwrap();
                        if !mem.get_bg_registers().in_fblank() {
                            send_reply.send(()).unwrap();
                            renderer.setup_caches(&mut mem);
                            let mut t = target.as_ref().unwrap().lock().unwrap();
                            renderer.draw_line(&mem, &mut t, y);
                        } else {
                            send_reply.send(()).unwrap();
                        }
                    }
                }
            }
        });

        RenderThread {
            sender:     send_msg,
            receiver:   recv_reply,
        }
    }

    pub fn start_frame(&mut self, target: RenderTarget) {
        self.sender
            .send(RendererMessage::StartFrame(target))
            .expect("Couldn't send start frame message!");
    }

    pub fn draw_line(&mut self, y: usize) {
        self.sender
            .send(RendererMessage::DrawLine(y))
            .expect("Couldn't send draw line message!");

        self.receiver
            .recv()
            .expect("Draw line");
    }
}