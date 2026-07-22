//! Pixgine Engine - A custom 2D pixel art game engine
//!
//! Architecture:
//! - `core` - Engine configuration, virtual resolution, window settings
//! - `renderer` - wgpu-based pixel-perfect rendering pipeline
//! - `ecs` - ECS component definitions using bevy_ecs
//! - `physics` - rapier2d physics integration
//! - `scripting` - Lua scripting with mlua
//! - `assets` - Asset loading, caching, and hot reloading
//! - `audio` - Sound effect and music playback
//! - `input` - Input handling abstraction
//! - `scene` - Scene loading/saving/serialization

pub mod core;
pub mod renderer;
pub mod ecs;
pub mod physics;
pub mod scripting;
pub mod assets;
pub mod audio;
pub mod input;
pub mod scene;

// Re-export commonly used types
pub use core::*;
pub use ecs::*;