use serde::{Deserialize, Serialize};

/// Virtual resolution for pixel-perfect rendering.
/// All game logic operates in these pixel coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualResolution {
    pub width: u32,
    pub height: u32,
}

impl Default for VirtualResolution {
    fn default() -> Self {
        Self {
            width: 320,
            height: 180,
        }
    }
}

/// Engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Virtual resolution for game logic and rendering
    pub virtual_resolution: VirtualResolution,
    /// Target window width (if not fullscreen)
    pub window_width: u32,
    /// Target window height (if not fullscreen)
    pub window_height: u32,
    /// Window title
    pub window_title: String,
    /// Enable vsync
    pub vsync: bool,
    /// Target FPS (0 = uncapped)
    pub target_fps: u32,
    /// Assets root directory
    pub assets_dir: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        let vr = VirtualResolution::default();
        Self {
            window_width: vr.width * 6,
            window_height: vr.height * 6,
            window_title: "Pixgine".to_string(),
            vsync: true,
            target_fps: 60,
            assets_dir: "assets".to_string(),
            virtual_resolution: vr,
        }
    }
}