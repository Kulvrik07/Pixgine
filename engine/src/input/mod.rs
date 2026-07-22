//! Input handling abstraction
//!
//! Processes raw winit input events and provides a clean API
//! for querying keyboard, mouse, and gamepad state.

mod manager;

pub use manager::*;