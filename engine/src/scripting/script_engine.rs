use anyhow::Result;
use mlua::{Function, Lua, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::scripting::EngineAPI;
use crate::input::InputManager;

/// The Lua scripting engine that manages script execution.
pub struct ScriptEngine {
    lua: Lua,
    scripts: HashMap<String, ScriptState>,
    script_dir: PathBuf,
}

struct ScriptState {
    source: String,
    last_modified: std::time::SystemTime,
    update_fn: Option<Function>,
    init_fn: Option<Function>,
}

impl ScriptEngine {
    pub fn new(assets_dir: &Path) -> Result<Self> {
        let lua = Lua::new();

        let engine = Self {
            lua,
            scripts: HashMap::new(),
            script_dir: assets_dir.join("scripts"),
        };

        Ok(engine)
    }

    /// Register built-in API bindings for Lua scripts
    pub fn register_api(&self, input: &InputManager) -> Result<()> {
        let api = EngineAPI::new(input);
        api.register(&self.lua)?;
        Ok(())
    }

    /// Load a Lua script file
    pub fn load_script(&mut self, path: &str) -> Result<()> {
        let full_path = self.script_dir.join(path);
        let source = std::fs::read_to_string(&full_path)?;
        let metadata = std::fs::metadata(&full_path)?;
        let last_modified = metadata.modified()?;

        // Execute the script to register functions
        let chunk = self.lua.load(&source).set_name(path);
        let _result: Value = chunk.eval()?;

        // Extract update and init functions if they exist
        let globals = self.lua.globals();
        let init_fn = globals.get::<Option<Function>>("init")?;
        let update_fn = globals.get::<Option<Function>>("update")?;

        // Call init if it exists (before moving into state)
        if let Some(ref init) = init_fn {
            init.call::<()>(())?;
        }

        self.scripts.insert(
            path.to_string(),
            ScriptState {
                source,
                last_modified,
                init_fn,
                update_fn,
            },
        );

        log::info!("Loaded script: {}", path);
        Ok(())
    }

    /// Check if any scripts have been modified and hot reload them
    pub fn hot_reload(&mut self) -> Result<Vec<String>> {
        let mut reloaded = Vec::new();

        for (name, state) in &self.scripts {
            let full_path = self.script_dir.join(name);
            if let Ok(metadata) = std::fs::metadata(&full_path) {
                if let Ok(modified) = metadata.modified() {
                    if modified > state.last_modified {
                        reloaded.push(name.clone());
                    }
                }
            }
        }

        for name in &reloaded {
            log::info!("Hot-reloading script: {}", name);
            self.load_script(name)?;
        }

        Ok(reloaded)
    }

    /// Call the update function on all scripts
    pub fn update_all(&self, dt: f32) -> Result<()> {
        let globals = self.lua.globals();
        globals.set("dt", dt)?;

        for (_name, state) in &self.scripts {
            if let Some(ref update) = state.update_fn {
                update.call::<()>(dt)?;
            }
        }

        Ok(())
    }
}