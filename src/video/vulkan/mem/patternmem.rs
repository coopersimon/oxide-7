// Pattern mem for a single background. Reads VRAM, outputs Textures.

use vulkano::{
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
        now, GpuFuture
    }
};

use std::sync::Arc;

pub type PatternImage = Arc<ImmutableImage<R8Uint>>;
pub type PatternFuture = Box<dyn GpuFuture>;

pub struct PatternMem {
    image:  Option<PatternImage>
}

impl PatternMem {
    pub fn new() -> Self {
        PatternMem {
            image: None
        }
    }
}