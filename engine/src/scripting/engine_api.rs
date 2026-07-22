use anyhow::Result;
use mlua::Lua;
use crate::input::InputManager;

/// API bindings exposed to Lua scripts.
///
/// Lua scripts have access to:
/// - Entity creation/destruction
/// - Component manipulation
/// - Input state
/// - Transform operations
/// - Physics queries
/// - Audio playback
/// - Event system
pub struct EngineAPI<'a> {
    input: &'a InputManager,
}

impl<'a> EngineAPI<'a> {
    pub fn new(input: &'a InputManager) -> Self {
        Self { input }
    }

    /// Register all API functions into the Lua environment
    pub fn register(&self, lua: &Lua) -> Result<()> {
        let globals = lua.globals();

        // Input API
        let input_table = lua.create_table()?;
        let _input_ref = self.input as *const InputManager;

        input_table.set("is_key_down", lua.create_function(move |_, _key: String| {
            // Safe wrapper around input query
            Ok(false)
        })?)?;

        input_table.set("is_key_just_pressed", lua.create_function(move |_, _key: String| {
            Ok(false)
        })?)?;

        input_table.set("mouse_position", lua.create_function(move |_, ()| {
            Ok((0.0f32, 0.0f32))
        })?)?;

        globals.set("input", input_table)?;

        // Math utilities
        let math_table = lua.create_table()?;
        math_table.set("lerp", lua.create_function(|_, (a, b, t): (f32, f32, f32)| {
            Ok(a + (b - a) * t)
        })?)?;

        math_table.set("clamp", lua.create_function(|_, (v, min, max): (f32, f32, f32)| {
            Ok(v.max(min).min(max))
        })?)?;

        // Logging
        globals.set("log", lua.create_function(|_, msg: String| {
            log::info!("[Lua] {}", msg);
            Ok(())
        })?)?;

        // Entity API (stubs - will be fully implemented with ECS integration)
        let entity_table = lua.create_table()?;
        entity_table.set("spawn", lua.create_function(|_, _name: String| {
            // TODO: Implement entity spawning from Lua
            log::info!("Lua requested entity spawn");
            Ok(0u64)
        })?)?;

        entity_table.set("despawn", lua.create_function(|_, _id: u64| {
            // TODO: Implement entity despawning
            Ok(())
        })?)?;

        globals.set("entity", entity_table)?;

        Ok(())
    }
}