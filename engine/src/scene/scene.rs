use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A serializable scene descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDescriptor {
    pub name: String,
    pub entities: Vec<EntityDescriptor>,
    #[serde(default)]
    pub tilemap: TilemapData,
}

/// Tilemap data for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilemapData {
    pub layers: Vec<TileLayerData>,
    pub tilesheet_tex_id: Option<u64>,
    pub tilesheet_cols: u32,
    pub tilesheet_rows: u32,
    pub tilesheet_tile_w: u32,
    pub tilesheet_tile_h: u32,
    pub palette: Vec<[f32; 4]>,
}

impl Default for TilemapData {
    fn default() -> Self {
        Self {
            layers: Vec::new(),
            tilesheet_tex_id: None,
            tilesheet_cols: 1,
            tilesheet_rows: 1,
            tilesheet_tile_w: 16,
            tilesheet_tile_h: 16,
            palette: Vec::new(),
        }
    }
}

/// A serializable tile layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileLayerData {
    pub name: String,
    pub visible: bool,
    pub tiles: Vec<Vec<u32>>,
    pub cols: usize,
    pub rows: usize,
    pub tile_size: u32,
    #[serde(default)]
    pub z_index: i32,
    #[serde(default)]
    pub spritesheet_tex_id: Option<u64>,
}

/// A serializable entity definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDescriptor {
    pub name: String,
    pub components: HashMap<String, serde_json::Value>,
}

/// Loaded scene with ECS entities
pub struct Scene {
    pub name: String,
    pub entities: Vec<EntityDescriptor>,
    pub entity_map: HashMap<String, Entity>,
}

impl Scene {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            entities: Vec::new(),
            entity_map: HashMap::new(),
        }
    }

    /// Load a scene from a JSON file
    pub fn load_from_file(path: &PathBuf) -> anyhow::Result<SceneDescriptor> {
        let content = std::fs::read_to_string(path)?;
        let descriptor: SceneDescriptor = serde_json::from_str(&content)?;
        Ok(descriptor)
    }

    /// Save the scene to a JSON file
    pub fn save_to_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        let descriptor = SceneDescriptor {
            name: self.name.clone(),
            entities: self.entities.clone(),
            tilemap: TilemapData::default(),
        };
        let content = serde_json::to_string_pretty(&descriptor)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}