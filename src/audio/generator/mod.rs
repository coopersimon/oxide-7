// The thread that deals with outputting audio.
mod resampler;

use crossbeam_channel::Receiver;

use std::thread;

pub struct AudioGenerator {
    thread: thread::JoinHandle<()>
}

impl AudioGenerator {
    pub fn new(rx: Receiver<super::SamplePacket>) -> Self {
        use cpal::traits::{
            HostTrait,
            DeviceTrait,
            EventLoopTrait
        };
    
        let thread = thread::spawn(move || {
            let host = cpal::default_host();
    
            let event_loop = host.event_loop();
    
            let device = host.default_output_device().expect("no output device available.");

            /*for f in device.supported_output_formats().unwrap() {
                println!("Format: {:?}", f);
            }*/
    
            //let mut supported_formats_range = device.supported_output_formats()
            //    .expect("error while querying formats");
    
            let format = pick_output_format(&device)
                .with_max_sample_rate();
    
            let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
    
            let sample_rate = format.sample_rate.0;

            let mut resampler = resampler::Resampler::new(rx, sample_rate as f64);
    
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
                        for (out, frame) in buffer.chunks_exact_mut(2).zip(&mut resampler) {
                            for (elem, f) in out.iter_mut().zip(frame.iter()) {
                                *elem = (f * u16::max_value() as f32) as u16
                            }
                        }
                    },
                    Output { buffer: I16(mut buffer) } => {
                        for (out, frame) in buffer.chunks_exact_mut(2).zip(&mut resampler) {
                            for (elem, f) in out.iter_mut().zip(frame.iter()) {
                                *elem = (f * i16::max_value() as f32) as i16
                            }
                        }
                    },
                    Output { buffer: F32(mut buffer) } => {
                        for (out, frame) in buffer.chunks_exact_mut(2).zip(&mut resampler) {
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

fn pick_output_format(device: &cpal::Device) -> cpal::SupportedFormat {
    use cpal::traits::DeviceTrait;

    const MIN: u32 = 32_000;

    let supported_formats_range = device.supported_output_formats()
        .expect("error while querying formats");

    for format in supported_formats_range {
        let cpal::SampleRate(v) = format.max_sample_rate;
        if v >= MIN {
            return format;
        }
    }

    device.supported_output_formats()
        .expect("error while querying formats")
        .next()
        .expect("No supported format")
}