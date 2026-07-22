//! Pixel-perfect rendering pipeline using wgpu.
//!
//! This module handles:
//! - Virtual resolution framebuffer
//! - Nearest-neighbor upscaling
//! - Sprite batching
//! - Texture atlas management
//! - Camera system integration

mod context;
mod pipeline;
mod sprite_batch;
mod texture;
mod camera;

pub use context::*;
pub use pipeline::*;
pub use sprite_batch::*;
pub use texture::*;
pub use camera::*;
pub use crate::core::PixelScale;
