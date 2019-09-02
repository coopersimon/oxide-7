// Pattern mem for a single background. Reads VRAM, outputs Textures.

use vulkano::{
    command_buffer::{
        AutoCommandBuffer,
        CommandBufferExecFuture
    },
    device::{
        Device,
        Queue
    },
    image::{
        Dimensions,
        immutable::ImmutableImage
    },
    format::{
        R8Uint
    },
    sync::{
        now, GpuFuture, NowFuture
    }
};

use std::sync::Arc;

use super::super::super::VideoMem;

pub type PatternImage = Arc<ImmutableImage<R8Uint>>;
pub type PatternFuture = Box<dyn GpuFuture>;

pub enum BitsPerPixel {
    _2,
    _4,
    _8
}

pub struct PatternMem {
    // Parameters
    width:          u32,
    height:         u32,
    bits_per_pixel: BitsPerPixel,

    start_addr:     u16,
    end_addr:       u16,

    // Vulkan
    device:         Arc<Device>,
    queue:          Arc<Queue>,
    image:          Option<PatternImage>
}

impl PatternMem {
    pub fn new(queue: &Arc<Queue>, device: &Arc<Device>, width: u32, height: u32, bits_per_pixel: BitsPerPixel, start_addr: u16) -> Self {
        let size = (width * height * match bits_per_pixel {
            BitsPerPixel::_2 => 16,
            BitsPerPixel::_4 => 32,
            BitsPerPixel::_8 => 64,
        }) as u16;   // TODO: check against max size

        PatternMem {
            width:          width,
            height:         height,
            bits_per_pixel: bits_per_pixel,

            start_addr:     start_addr,
            end_addr:       start_addr + size,

            device:         device.clone(),
            queue:          queue.clone(),
            image:          None
        }
    }

    // Call if VRAM is known to be dirty.
    pub fn clear_image(&mut self, mem: &mut VideoMem) {
        if mem.vram_dirty_range(self.start_addr, self.end_addr) {
            self.image = None;
        }
    }

    // Return cached image or create one if none is cached.
    pub fn get_image(&mut self, mem: &mut VideoMem) -> (PatternImage, PatternFuture) {
        if let Some(image) = &self.image {
            (image.clone(), Box::new(now(self.device.clone())))
        } else {
            let data = &mem.get_vram()[(self.start_addr as usize)..(self.end_addr as usize)];
            let (image, future) = self.make_image(data);
            self.image = Some(image.clone());
            (image, Box::new(future))
        }
    }
}

// Internal
impl PatternMem {
    // Make the image. Input raw data, width and height in PIXELS/TEXELS, and bits per pixel.
    // Rows are always 16 tiles. (16 x 8 = 128 pixels.)
    fn make_image(&mut self, data: &[u8]) -> (PatternImage, CommandBufferExecFuture<NowFuture, AutoCommandBuffer>) {
        let mut texture_data = vec![0; (self.width * self.height) as usize];

        let row_size = 16 * 8;      // Row length in pixels.
        let mut col = 0;            // Current col of tile in pixels.
        let mut row = 0;            // Current row of tile in pixels.
        let mut y = 0;              // Current Y coord in pixels. (To the nearest tile.)

        match self.bits_per_pixel {
            // 16 bytes per tile.
            BitsPerPixel::_2 => for (i, d) in data.iter().enumerate() {
                let bitplane = i % 2;
                let base_index = ((y + row) * row_size) + col;

                for x in 0..8 {
                    let bit = (d >> (7 - x)) & 1;
                    texture_data[base_index + x] |= bit << bitplane;
                }

                if bitplane != 0 {
                    // Next row in tile.
                    row += 1;

                    // Next tile.
                    if row >= 8 {
                        row = 0;

                        col += 8;
                        if col >= row_size {
                            col = 0;
                            y += 8;
                        }
                    }
                }
            },

            // 32 bytes per tile.
            BitsPerPixel::_4 => {
                let mut bitplane_offset = 0;    // As bitplanes are stored in pairs.

                for (i, d) in data.iter().enumerate() {
                    let sub_bitplane = i % 2;
                    let bitplane = sub_bitplane + bitplane_offset;
                    let base_index = ((y + row) * row_size) + col;

                    for x in 0..8 {
                        let bit = (*d >> (7 - x)) & 1;
                        texture_data[base_index + x] |= bit << bitplane;
                    }

                    if sub_bitplane != 0 {
                        // Next row in tile.
                        row += 1;
                        if row >= 8 {
                            row = 0;

                            bitplane_offset += 2;
                            // Next tile.
                            if bitplane_offset >= 4 {
                                bitplane_offset = 0;

                                col += 8;
                                if col >= row_size {
                                    col = 0;
                                    y += 8;
                                }
                            }
                        }
                    }

                }
            },

            // 64 bytes per tile.
            BitsPerPixel::_8 => {
                let mut x = 0;

                for d in data.iter() {
                    let base_index = ((y + row) * row_size) + col;

                    texture_data[base_index + x] = *d;

                    x += 1;
                    if x >= 8 {
                        // Next row in tile.
                        row += 1;
                        if row >= 8 {
                            row = 0;

                            // Next tile.
                            col += 8;
                            if col >= row_size {
                                col = 0;
                                y += 8;
                            }
                        }
                    }

                }
            },
        }
        
        ImmutableImage::from_iter(
            texture_data.drain(..),
            Dimensions::Dim2d { width: self.width, height: self.height },
            R8Uint,
            self.queue.clone()
        ).expect("Couldn't create image.")
    }
}