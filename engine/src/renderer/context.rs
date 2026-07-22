use crate::core::VirtualResolution;
use crate::renderer::{PixelPerfectPipeline, UpscalePipeline, SpriteBatch, TextureManager, Camera, PixelScale};
use anyhow::Result;

/// The main render context that owns all GPU resources.
///
/// Architecture:
/// 1. Sprites are rendered to a **virtual-resolution framebuffer** (e.g. 320×180)
///    using an orthographic projection in pixel coordinates.
/// 2. The framebuffer is upscaled to the window using **nearest-neighbor**
///    filtering with an integer scale factor, guaranteeing pixel-perfect output.
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
    /// Virtual-resolution framebuffer texture
    framebuffer: wgpu::Texture,
    /// View of the framebuffer (sampled by the upscale pass)
    framebuffer_view: wgpu::TextureView,
    /// Upscale pipeline (framebuffer → window, nearest-neighbor)
    upscale: UpscalePipeline,
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

        // --- Virtual-resolution framebuffer ---
        let framebuffer = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Virtual Resolution Framebuffer"),
            size: wgpu::Extent3d {
                width: virtual_res.width,
                height: virtual_res.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let framebuffer_view = framebuffer.create_view(&wgpu::TextureViewDescriptor::default());

        let pipeline = PixelPerfectPipeline::new(&device, format, &virtual_res);
        let sprite_batch = SpriteBatch::new(&device, &virtual_res);
        let textures = TextureManager::new(&device, &queue);
        let camera = Camera::new(&virtual_res);
        let upscale = UpscalePipeline::new(
            &device,
            format,
            &framebuffer_view,
            &virtual_res,
            window_width,
            window_height,
        );

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
            framebuffer,
            framebuffer_view,
            upscale,
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
        // Recalculate integer pixel scale for the upscale pass
        self.upscale.resize(&self.virtual_res, width, height);
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

        // --- Pass 1: Render sprites to the virtual-resolution framebuffer ---
        // Flush sprite batch and render
        self.sprite_batch.flush(&self.device, &self.queue);

        self.pipeline.render(
            &self.queue,
            &mut encoder,
            &self.framebuffer_view,
            &self.sprite_batch,
            &self.camera,
            &self.virtual_res,
        );

        // --- Pass 2: Upscale framebuffer → window (nearest-neighbor) ---
        self.upscale.render(
            &mut encoder,
            &view,
            &self.virtual_res,
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        // Present the frame to the screen
        frame.present();
        Ok(())
    }
}
