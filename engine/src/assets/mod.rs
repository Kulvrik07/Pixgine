//! Asset loading, caching, and hot reloading system.
//!
//! The asset pipeline handles:
//! - Loading textures, audio, scenes, scripts, and tilemaps
//! - Asset handle generation and caching
//! - Hot reloading when files change on disk

mod manager;
mod handle;
mod watcher;

pub use manager::*;
pub use handle::*;
pub use watcher::*;