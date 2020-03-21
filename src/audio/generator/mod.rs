// The thread that deals with outputting audio.
mod envelope;
mod internal;
mod types;
mod voicegen;

use crossbeam_channel::Receiver;

use std::thread;

use internal::InternalAudioGenerator;
pub use types::*;

pub struct AudioGenerator {
    thread: thread::JoinHandle<()>
}

impl AudioGenerator {
    pub fn new(rx: Receiver<AudioData>) -> Self {
        use cpal::traits::{
            HostTrait,
            DeviceTrait,
            EventLoopTrait
        };
    
        let thread = thread::spawn(move || {
            let host = cpal::default_host();
    
            let event_loop = host.event_loop();
    
            let device = host.default_output_device().expect("no output device available.");
    
            let mut supported_formats_range = device.supported_output_formats()
                .expect("error while querying formats");
    
            let format = supported_formats_range.next()
                .expect("No supported format")
                .with_max_sample_rate();
    
            let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
    
            let sample_rate = format.sample_rate.0 as usize;
    
            let mut generator = InternalAudioGenerator::new(rx, sample_rate);
    
            event_loop.play_stream(stream_id).expect("Stream could not start.");
    
            event_loop.run(move |_stream_id, stream_result| {
                use cpal::StreamData::*;
                use cpal::UnknownTypeOutputBuffer::*;
    
                let stream_data = match stream_result {
                    Ok(data) => data,
                    Err(e) => {
                        eprintln!("An error occurred in audio generator: {}", e);
                        return;
                    }
                };
    
                match stream_data {
                    Output { buffer: U16(mut buffer) } => {
                        for out in buffer.chunks_exact_mut(2) {
                            let frame = generator.process_frame();
                            for (elem, f) in out.iter_mut().zip(frame.iter()) {
                                *elem = (f * u16::max_value() as f32) as u16
                            }
                        }
                    },
                    Output { buffer: I16(mut buffer) } => {
                        for out in buffer.chunks_exact_mut(2) {
                            let frame = generator.process_frame();
                            for (elem, f) in out.iter_mut().zip(frame.iter()) {
                                *elem = (f * i16::max_value() as f32) as i16
                            }
                        }
                    },
                    Output { buffer: F32(mut buffer) } => {
                        for out in buffer.chunks_exact_mut(2) {
                            let frame = generator.process_frame();
                            for (elem, f) in out.iter_mut().zip(frame.iter()) {
                                *elem = *f;
                            }
                        }
                    },
                    _ => {},
                }
            });
        });

        AudioGenerator {
            thread: thread
        }
    }
}
