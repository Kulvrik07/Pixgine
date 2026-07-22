use crate::core::EngineConfig;
use crate::renderer::RenderContext;
use crate::input::InputManager;

/// Main engine struct that owns all subsystems.
pub struct Engine {
    pub config: EngineConfig,
    pub renderer: Option<RenderContext>,
    pub input: InputManager,
    pub running: bool,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            renderer: None,
            input: InputManager::new(),
            running: true,
        }
    }

    pub fn init_renderer(
        &mut self,
        window: &winit::window::Window,
        surface: wgpu::Surface<'static>,
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
    ) {
        let size = window.inner_size();
        let renderer = RenderContext::new(
            surface,
            adapter,
            device,
            queue,
            self.config.virtual_resolution.clone(),
            size.width,
            size.height,
        );
        self.renderer = Some(renderer);
    }
}