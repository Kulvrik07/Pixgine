use crate::core::VirtualResolution;
use crate::renderer::{PixelPerfectPipeline, SpriteBatch, TextureManager, Camera};
use anyhow::Result;

/// The main render context that owns all GPU resources.
pub struct RenderContext {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,
    pub config: wgpu::SurfaceConfiguration,
    pub virtual_res: VirtualResolution,
    pub pipeline: PixelPerfectPipeline,
    pub sprite_batch: SpriteBatch,
    pub textures: TextureManager,
    pub camera: Camera,
    window_width: u32,
    window_height: u32,
}

impl RenderContext {
    pub fn new(
        surface: wgpu::Surface<'static>,
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
        virtual_res: VirtualResolution,
        window_width: u32,
        window_height: u32,
    ) -> Self {
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().find(|f| f.is_srgb()).copied()
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: window_width,
            height: window_height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let pipeline = PixelPerfectPipeline::new(&device, format, &virtual_res);
        let sprite_batch = SpriteBatch::new(&device, &virtual_res);
        let textures = TextureManager::new(&device, &queue);
        let camera = Camera::new(&virtual_res);

        Self {
            surface,
            device,
            queue,
            adapter,
            config,
            virtual_res,
            pipeline,
            sprite_batch,
            textures,
            camera,
            window_width,
            window_height,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.window_width = width;
        self.window_height = height;
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn begin_frame(&mut self) -> Option<wgpu::SurfaceTexture> {
        match self.surface.get_current_texture() {
            Ok(frame) => Some(frame),
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                // Retry once — needed on Wayland where the first frame may fail
                // while the compositor completes the configure handshake
                self.surface.get_current_texture().ok()
            }
            Err(wgpu::SurfaceError::Timeout) => {
                None
            }
            Err(e) => {
                log::error!("Surface error: {:?}", e);
                None
            }
        }
    }

    pub fn render(&mut self, frame: wgpu::SurfaceTexture) -> Result<()> {
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Clear the framebuffer
        let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Clear Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.9,
                        g: 0.2,
                        b: 0.2,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Flush sprite batch and render
        self.sprite_batch.flush(&self.device, &self.queue);

        // Render sprites through the pixel-perfect pipeline
        self.pipeline.render(
            &self.queue,
            &mut encoder,
            &view,
            &self.sprite_batch,
            &self.camera,
            &self.virtual_res,
            self.window_width,
            self.window_height,
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        // Present the frame to the screen
        frame.present();
        Ok(())
    }
}
