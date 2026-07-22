//! Pixgine Game Runtime
//!
//! The standalone game application that loads and runs games
//! built with the Pixgine engine.

use std::sync::Arc;
use anyhow::Result;
use pixgine_engine::core::{Engine, EngineConfig};
use pixgine_engine::ecs::*;
use pixgine_engine::input::InputManager;
use winit::application::ApplicationHandler;
use winit::event::*;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

struct GameApp {
    engine: Option<Engine>,
    window: Option<Arc<Window>>,
    world: Option<bevy_ecs::world::World>,
    schedule: Option<bevy_ecs::schedule::Schedule>,
    last_frame: std::time::Instant,
    config: EngineConfig,
}

impl ApplicationHandler for GameApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Pixgine Engine v{} starting on Wayland", env!("CARGO_PKG_VERSION"));
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        self.last_frame = std::time::Instant::now();

        // On Wayland/X11, window must be created inside resumed()
        if self.window.is_some() {
            log::info!("Window already initialized, skipping resumed()");
            return;
        }

        log::info!("Creating window in resumed() callback");

        let window_attrs = Window::default_attributes()
            .with_title(&self.config.window_title)
            .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
            .with_visible(true)
            .with_active(true)
            .with_window_icon(None);
        let window = event_loop.create_window(window_attrs).unwrap();
        // Explicitly request visibility + focus (needed on Wayland)
        window.set_visible(true);
        window.focus_window();

        // Initialize wgpu
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // SAFETY: Leak window so wgpu Surface<'static> can reference it for the program's lifetime
        let window = Arc::new(window);
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ))
        .unwrap();

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Pixgine Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .unwrap();

        let mut engine = Engine::new(self.config.clone());
        engine.init_renderer(&window, surface, adapter, device, queue);

        // Build ECS world
        let mut world = bevy_ecs::world::World::new();
        let schedule = build_core_schedule();

        world.insert_resource(TimeResource::default());
        world.insert_resource(InputResource {
            manager: InputManager::new(),
        });

        self.engine = Some(engine);
        self.window = Some(window);
        self.world = Some(world);
        self.schedule = Some(schedule);

        log::info!("Pixgine window created and initialized");

        // Force initial redraw — needed on Wayland to commit first buffer
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let engine = self.engine.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => {
                engine.running = false;
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Update frame timing
                let now = std::time::Instant::now();
                let dt = now.duration_since(self.last_frame).as_secs_f32();
                self.last_frame = now;

                // Game loop tick
                if let Some(ref mut world) = self.world {
                    if let Some(ref mut schedule) = self.schedule {
                        if let Some(mut time) = world.get_resource_mut::<TimeResource>() {
                            time.delta = dt;
                            time.elapsed += dt;
                            time.frame_count += 1;
                        }
                        schedule.run(world);
                    }
                }

                // Render
                if let Some(ref mut renderer) = engine.renderer {
                    if let Some(frame) = renderer.begin_frame() {
                        let _ = renderer.render(frame);
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => {
                match state {
                    ElementState::Pressed => engine.input.handle_key_down(key),
                    ElementState::Released => engine.input.handle_key_up(key),
                }
            }
            WindowEvent::Resized(size) => {
                log::info!("Window resized to {}x{}", size.width, size.height);
                if let Some(ref mut renderer) = engine.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Pixgine Game Runtime");

    let config = EngineConfig {
        window_title: "Pixgine Game".to_string(),
        ..Default::default()
    };

    let event_loop = EventLoop::new().unwrap();

    let mut app = GameApp {
        engine: None,
        window: None,
        world: None,
        schedule: None,
        last_frame: std::time::Instant::now(),
        config,
    };

    event_loop.run_app(&mut app).unwrap();

    Ok(())
}