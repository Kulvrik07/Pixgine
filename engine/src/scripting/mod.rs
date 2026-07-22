//! Lua scripting system using mlua.
//!
//! Provides a safe API boundary between Rust and Lua scripts.
//! Lua controls gameplay behavior with access to entities,
//! components, input, physics, audio, and more.

mod engine_api;
mod script_engine;

pub use engine_api::*;
pub use script_engine::*;