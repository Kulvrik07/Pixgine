use std::collections::HashMap;
use image::GenericImageView;

/// Manages texture loading and GPU resource creation
pub struct TextureManager {
    textures: HashMap<u64, TextureResource>,
    next_id: u64,
    sampler: wgpu::Sampler,
}

pub struct TextureResource {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl TextureManager {
    pub fn new(device: &wgpu::Device, _queue: &wgpu::Queue) -> Self {
        // Nearest-neighbor sampler for pixel art
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Pixel Art Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            textures: HashMap::new(),
            next_id: 1,
            sampler,
        }
    }

    pub fn get_sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// Load a texture from raw pixel data
    pub fn load_from_bytes(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
    ) -> Result<u64, String> {
        let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Texture {}", self.next_id)),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let id = self.next_id;
        self.next_id += 1;

        self.textures.insert(
            id,
            TextureResource {
                texture,
                view,
                width: dimensions.0,
                height: dimensions.1,
            },
        );

        Ok(id)
    }

    pub fn get_texture(&self, id: u64) -> Option<&TextureResource> {
        self.textures.get(&id)
    }

    pub fn get_view(&self, id: u64) -> Option<&wgpu::TextureView> {
        self.textures.get(&id).map(|t| &t.view)
    }
}