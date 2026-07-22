use bevy_ecs::prelude::*;
use crate::core::EngineConfig;
use crate::input::InputManager;
use crate::renderer::RenderContext;

/// Global ECS resources

/// Input resource - updated each frame before systems run
#[derive(Resource)]
pub struct InputResource {
    pub manager: InputManager,
}

/// Time resource - tracks frame timing
#[derive(Resource)]
pub struct TimeResource {
    pub delta: f32,
    pub elapsed: f32,
    pub frame_count: u64,
}

impl Default for TimeResource {
    fn default() -> Self {
        Self {
            delta: 0.0,
            elapsed: 0.0,
            frame_count: 0,
        }
    }
}

/// Engine configuration resource
#[derive(Resource)]
pub struct EngineConfigResource(pub EngineConfig);

/// Render context resource (available after initialization)
#[derive(Resource)]
pub struct RenderResource {
    pub context: RenderContext,
}

/// View state that follows CameraTag entity
#[derive(Resource, Default)]
pub struct ViewState {
    pub camera_x: f32,
    pub camera_y: f32,
    pub camera_follow: bool,
}