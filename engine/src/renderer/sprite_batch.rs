use crate::renderer::pipeline::SpriteVertex;
use crate::core::VirtualResolution;
use std::collections::HashMap;

type TextureId = u64;

/// Manages sprite batching, grouping sprites by texture to minimize draw calls.
pub struct SpriteBatch {
    pub groups: HashMap<TextureId, (wgpu::Buffer, u32)>, // (buffer, vertex_count)
    pub texture_bindings: HashMap<TextureId, (wgpu::BindGroup, wgpu::Sampler)>,
    pending_sprites: Vec<SpriteDraw>,
    batch_dirty: bool,
}

/// A pending sprite to be batched
#[derive(Debug, Clone)]
pub struct SpriteDraw {
    pub texture_id: TextureId,
    pub vertices: [SpriteVertex; 4],
    pub layer: f32,
}

impl SpriteBatch {
    pub fn new(_device: &wgpu::Device, _virtual_res: &VirtualResolution) -> Self {
        Self {
            groups: HashMap::new(),
            texture_bindings: HashMap::new(),
            pending_sprites: Vec::new(),
            batch_dirty: false,
        }
    }

    /// Queue a sprite for rendering
    pub fn draw_sprite(&mut self, draw: SpriteDraw) {
        self.pending_sprites.push(draw);
        self.batch_dirty = true;
    }

    /// Build vertex buffers from pending sprites
    pub fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if !self.batch_dirty {
            return;
        }

        // Group vertices by texture
        let mut grouped: HashMap<TextureId, Vec<SpriteVertex>> = HashMap::new();
        for draw in self.pending_sprites.drain(..) {
            grouped.entry(draw.texture_id).or_default().extend(draw.vertices);
        }

        // Create/update buffers for each texture group
        for (texture_id, vertices) in grouped {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Sprite Batch {}", texture_id)),
                size: (vertices.len() * std::mem::size_of::<SpriteVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&vertices));
            self.groups.insert(texture_id, (buffer, vertices.len() as u32));
        }

        self.batch_dirty = false;
    }

    /// Register a texture binding group for a texture ID
    pub fn register_texture(
        &mut self,
        device: &wgpu::Device,
        texture_id: TextureId,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) {
        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(&format!("Texture Bind Group Layout {}", texture_id)),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("Texture Bind Group {}", texture_id)),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        self.texture_bindings.insert(texture_id, (bind_group, sampler.clone()));
    }
}