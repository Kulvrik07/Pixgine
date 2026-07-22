use crate::core::VirtualResolution;

/// Represents the scale factor between virtual resolution and actual window size.
#[derive(Debug, Clone, Copy)]
pub struct PixelScale {
    pub factor: u32,
    pub offset_x: u32,
    pub offset_y: u32,
}

impl PixelScale {
    /// Calculate the integer scale factor and centering offsets
    /// to fit the virtual resolution within the actual window size.
    pub fn calculate(virtual_res: &VirtualResolution, window_width: u32, window_height: u32) -> Self {
        let scale_x = window_width / virtual_res.width;
        let scale_y = window_height / virtual_res.height;
        let factor = scale_x.min(scale_y).max(1);

        let scaled_width = virtual_res.width * factor;
        let scaled_height = virtual_res.height * factor;

        let offset_x = (window_width - scaled_width) / 2;
        let offset_y = (window_height - scaled_height) / 2;

        Self {
            factor,
            offset_x,
            offset_y,
        }
    }

    /// Convert a screen-space pixel position to virtual resolution coordinates.
    pub fn screen_to_virtual(&self, screen_x: f32, screen_y: f32) -> (f32, f32) {
        let vx = (screen_x - self.offset_x as f32) / self.factor as f32;
        let vy = (screen_y - self.offset_y as f32) / self.factor as f32;
        (vx, vy)
    }
}