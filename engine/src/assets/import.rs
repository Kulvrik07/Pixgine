//! Import settings for assets — modelled after Godot's import dock.
//!
//! Each imported texture gets an [`ImportSettings`] record that controls
//! how it is interpreted by the engine:
//!
//! - **`Texture`** – a plain single-image texture (no auto-slicing).
//! - **`Spritesheet`** – the image is divided into a uniform grid of tiles
//!   whose dimensions are specified by `tile_width` / `tile_height`.
//!
//! Settings are persisted per-asset so that re-importing a texture with
//! different settings produces the expected result without re-creating
//! the asset from scratch.

use serde::{Deserialize, Serialize};

/// How a texture should be interpreted after import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextureType {
    /// A single image — the full texture is used as one quad.
    Texture,
    /// A uniform grid of tiles (spritesheet / atlas).
    Spritesheet,
}

impl Default for TextureType {
    fn default() -> Self {
        TextureType::Texture
    }
}

impl TextureType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TextureType::Texture => "Texture",
            TextureType::Spritesheet => "Spritesheet",
        }
    }
}

/// Filtering mode applied when sampling the texture on the GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextureFilter {
    /// Nearest-neighbour — crisp pixel art, no blur.
    Nearest,
    /// Linear interpolation — smooth, blurry scaling.
    Linear,
}

impl Default for TextureFilter {
    fn default() -> Self {
        TextureFilter::Nearest
    }
}

/// Per-asset import configuration.
///
/// This replaces the old global `texture_only_import` checkbox with
/// fine-grained, per-texture control — exactly like Godot's Import dock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSettings {
    /// How the texture is interpreted (plain texture or spritesheet).
    pub texture_type: TextureType,
    /// Tile width in pixels (only meaningful for `Spritesheet`).
    pub tile_width: u32,
    /// Tile height in pixels (only meaningful for `Spritesheet`).
    pub tile_height: u32,
    /// GPU sampling filter.
    pub filter: TextureFilter,
    /// Whether the texture should repeat (wrap) or clamp to edges.
    pub repeat: bool,
    /// Whether to generate mipmaps.
    pub mipmaps: bool,
}

impl Default for ImportSettings {
    fn default() -> Self {
        Self {
            texture_type: TextureType::Texture,
            tile_width: 16,
            tile_height: 16,
            filter: TextureFilter::Nearest,
            repeat: false,
            mipmaps: false,
        }
    }
}

impl ImportSettings {
    /// Create settings for a plain texture (no slicing).
    pub fn texture() -> Self {
        Self::default()
    }

    /// Create settings for a spritesheet with the given tile dimensions.
    pub fn spritesheet(tile_w: u32, tile_h: u32) -> Self {
        Self {
            texture_type: TextureType::Spritesheet,
            tile_width: tile_w.max(1),
            tile_height: tile_h.max(1),
            ..Default::default()
        }
    }

    /// Auto-detect a reasonable tile size from the texture dimensions.
    ///
    /// Tries common power-of-two sizes (64, 32, 16, 8) and falls back
    /// to 16×16 if none divide evenly.
    pub fn auto_tile_size(tex_w: u32, tex_h: u32) -> u32 {
        for &size in &[64, 32, 16, 8] {
            if tex_w >= size && tex_h >= size && tex_w % size == 0 && tex_h % size == 0 {
                return size;
            }
        }
        16.min(tex_w).min(tex_h).max(1)
    }

    /// Convenience: create spritesheet settings with auto-detected tile size.
    pub fn auto_spritesheet(tex_w: u32, tex_h: u32) -> Self {
        let s = Self::auto_tile_size(tex_w, tex_h);
        Self::spritesheet(s, s)
    }

    /// Number of tile columns.
    pub fn cols(&self, tex_w: u32) -> u32 {
        (tex_w / self.tile_width).max(1)
    }

    /// Number of tile rows.
    pub fn rows(&self, tex_h: u32) -> u32 {
        (tex_h / self.tile_height).max(1)
    }

    /// Total number of tiles.
    pub fn tile_count(&self, tex_w: u32, tex_h: u32) -> u32 {
        self.cols(tex_w) * self.rows(tex_h)
    }
}
