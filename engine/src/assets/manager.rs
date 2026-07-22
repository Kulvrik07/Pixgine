use std::collections::HashMap;
use crate::assets::handle::AssetHandle;
use crate::assets::watcher::FileWatcher;
use crate::renderer::TextureManager;
use anyhow::Result;
use std::path::PathBuf;

/// Type of asset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
    Texture,
    Audio,
    Scene,
    Script,
    Tileset,
    Animation,
    Font,
}

/// Metadata for a loaded asset
#[derive(Debug, Clone)]
pub struct AssetMeta {
    pub handle: AssetHandle,
    pub asset_type: AssetType,
    pub path: PathBuf,
    pub loaded: bool,
}

/// The main asset manager
pub struct AssetManager {
    textures: TextureManager,
    assets: HashMap<AssetHandle, AssetMeta>,
    path_to_handle: HashMap<PathBuf, AssetHandle>,
    file_watcher: Option<FileWatcher>,
    assets_dir: PathBuf,
    next_handle: u64,
}

impl AssetManager {
    pub fn new(assets_dir: &str, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let textures = TextureManager::new(device, queue);

        Self {
            textures,
            assets: HashMap::new(),
            path_to_handle: HashMap::new(),
            file_watcher: None,
            assets_dir: PathBuf::from(assets_dir),
            next_handle: 1,
        }
    }

    pub fn init_file_watcher(&mut self) -> Result<()> {
        let watcher = FileWatcher::new(&self.assets_dir)?;
        self.file_watcher = Some(watcher);
        Ok(())
    }

    pub fn textures_mut(&mut self) -> &mut TextureManager {
        &mut self.textures
    }

    pub fn textures(&self) -> &TextureManager {
        &self.textures
    }

    /// Load a texture from a path relative to assets directory
    pub fn load_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &str,
    ) -> Result<AssetHandle> {
        let full_path = self.assets_dir.join(path);
        let bytes = std::fs::read(&full_path)?;
        let _texture_id = self.textures.load_from_bytes(device, queue, &bytes)
            .map_err(|e| anyhow::anyhow!("Failed to load texture: {}", e))?;

        let handle = AssetHandle::new(self.next_handle);
        self.next_handle += 1;

        self.assets.insert(
            handle,
            AssetMeta {
                handle,
                asset_type: AssetType::Texture,
                path: full_path.clone(),
                loaded: true,
            },
        );
        self.path_to_handle.insert(full_path, handle);

        Ok(handle)
    }

    /// Check for file changes and handle hot reload
    pub fn poll_hot_reload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<PathBuf> {
        let Some(ref mut watcher) = self.file_watcher else {
            return Vec::new();
        };

        let changed = watcher.poll_changes();
        for path in &changed {
            if let Some(&handle) = self.path_to_handle.get(path) {
                if let Some(meta) = self.assets.get(&handle) {
                    match meta.asset_type {
                        AssetType::Texture => {
                            if let Ok(bytes) = std::fs::read(path) {
                                let _ = self.textures.load_from_bytes(device, queue, &bytes);
                                log::info!("Hot-reloaded texture: {:?}", path);
                            }
                        }
                        AssetType::Script => {
                            log::info!("Script changed: {:?}", path);
                        }
                        _ => {}
                    }
                }
            }
        }
        changed
    }
}