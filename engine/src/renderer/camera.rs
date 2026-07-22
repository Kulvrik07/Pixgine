use crate::core::VirtualResolution;
use glam::Mat4;

/// Simple 2D camera with pixel-aligned positioning
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

impl Camera {
    pub fn new(_virtual_res: &VirtualResolution) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }

    /// Build an orthographic projection matrix for the virtual resolution.
    /// Uses pixel-coordinate system where (0,0) is top-left.
    pub fn build_projection(&self, virtual_res: &VirtualResolution) -> [[f32; 4]; 4] {
        let left = self.x;
        let right = self.x + virtual_res.width as f32 / self.zoom;
        let top = self.y;
        let bottom = self.y + virtual_res.height as f32 / self.zoom;

        let projection = Mat4::orthographic_rh(left, right, bottom, top, -1000.0, 1000.0);
        projection.to_cols_array_2d()
    }
}