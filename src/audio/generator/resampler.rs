// Module that resamples from 32_000 to the output sample rate.
use crossbeam_channel::Receiver;
use sample::{
    frame::{Frame, Stereo},
    interpolate::{Converter, Sinc},
    ring_buffer::Fixed,
    signal::Signal
};

pub struct Resampler {
    converter: Converter<Source, Sinc<[Stereo<f32>; 16]>>
}

impl Resampler {
    pub fn new(receiver: Receiver<super::super::SamplePacket>, target_sample_rate: f64) -> Self {
        let sinc = Sinc::new(Fixed::from([Stereo::equilibrium(); 16]));
        Resampler {
            /*converter: Converter::from_hz_to_hz(
                Source::new(receiver),
                Sinc::new(Fixed::from([Stereo::equilibrium(); 1024])),
                32_000.0,
                target_sample_rate
            )*/
            converter: Source::new(receiver).from_hz_to_hz(sinc, 32_000.0, target_sample_rate)
        }
    }
}

impl Iterator for Resampler {
    type Item = Stereo<f32>;

    fn next(&mut self) -> Option<Self::Item> {
        let s = self.converter.next();
        //println!("{:?}", s);
        Some(s)
    }
}

// TODO: replace this with an async stream?
struct Source {
    receiver:   Receiver<super::super::SamplePacket>,

    current:    super::super::SamplePacket,
    n:          usize,
}

impl Source {
    fn new(receiver: Receiver<super::super::SamplePacket>) -> Self {
        Source {
            receiver:   receiver,

            current:    Box::new([]),
            n:          0,
        }
    }
}

impl Signal for Source {
    type Frame = Stereo<f32>;

    fn next(&mut self) -> Self::Frame {
        let x = if self.n < self.current.len() {
            let out = self.current[self.n];
            self.n += 1;
            out
        } else {
            let out = self.receiver.recv().unwrap();
            //println!("RECV");
            self.current = out;
            self.n = 1;
            self.current[0]
        };

        //println!("{} = {:?}", self.n, x);
        x
    }
}