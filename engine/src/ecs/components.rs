use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use glam::Vec2;

/// Transform component - position, rotation, scale in pixel coordinates
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub rotation: f32, // radians
    pub scale_x: f32,
    pub scale_y: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}

impl Transform {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            ..Default::default()
        }
    }

    pub fn position(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

/// Sprite component
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Sprite {
    /// Asset ID of the texture
    pub texture_id: Option<u64>,
    /// Source rectangle in the texture (for atlases/spritesheets)
    pub source_x: u32,
    pub source_y: u32,
    pub source_width: u32,
    pub source_height: u32,
    /// Tint color (RGBA)
    pub color: [f32; 4],
    /// Render layer (higher = on top)
    pub layer: i32,
    /// Whether the sprite is visible
    pub visible: bool,
    /// Flip horizontally
    pub flip_x: bool,
    /// Flip vertically
    pub flip_y: bool,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            texture_id: None,
            source_x: 0,
            source_y: 0,
            source_width: 0,
            source_height: 0,
            color: [1.0, 1.0, 1.0, 1.0],
            layer: 0,
            visible: true,
            flip_x: false,
            flip_y: false,
        }
    }
}

/// Animation component
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Animation {
    /// Animation ID/name
    pub name: String,
    /// Current frame index
    pub current_frame: usize,
    /// Frame timings in seconds
    pub frame_durations: Vec<f32>,
    /// Time since last frame change
    pub elapsed: f32,
    /// Whether the animation loops
    pub looping: bool,
    /// Whether the animation is playing
    pub playing: bool,
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            name: "idle".into(),
            current_frame: 0,
            frame_durations: vec![0.1],
            elapsed: 0.0,
            looping: true,
            playing: true,
        }
    }
}

/// Physics component
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Physics {
    pub body_type: PhysicsBodyType,
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
    /// Collision shape dimensions
    pub collider_width: f32,
    pub collider_height: f32,
    pub is_trigger: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PhysicsBodyType {
    Static,
    Dynamic,
    Kinematic,
}

impl Default for Physics {
    fn default() -> Self {
        Self {
            body_type: PhysicsBodyType::Static,
            mass: 1.0,
            friction: 0.3,
            restitution: 0.0,
            collider_width: 16.0,
            collider_height: 16.0,
            is_trigger: false,
        }
    }
}

/// Script component - links an entity to a Lua script
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Script {
    /// Path to the Lua script file (relative to assets/scripts/)
    pub path: String,
    /// The script source code
    pub source: String,
}

/// Tag component for player-controlled entities
#[derive(Component, Debug, Clone)]
pub struct Player;

/// Tag component for camera entities
#[derive(Component, Debug, Clone)]
pub struct CameraTag;

/// Velocity component for movement
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

impl Default for Velocity {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// Particle emitter component
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct ParticleEmitter {
    pub emitting: bool,
    pub max_particles: usize,
    pub spawn_rate: f32, // particles per second
    pub spawn_timer: f32,
    pub lifetime: f32, // particle lifetime in seconds
    pub speed: f32,
    pub speed_variance: f32,
    pub start_color: [f32; 4],
    pub end_color: [f32; 4],
    pub start_size: f32,
    pub end_size: f32,
    pub gravity: f32,
    pub particles: Vec<Particle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub lifetime: f32,
    pub age: f32,
    pub size: f32,
    pub color: [f32; 4],
}

impl PartialEq for ParticleEmitter {
    fn eq(&self, other: &Self) -> bool {
        self.emitting == other.emitting &&
        self.max_particles == other.max_particles &&
        self.spawn_rate == other.spawn_rate &&
        self.lifetime == other.lifetime &&
        self.speed == other.speed &&
        self.speed_variance == other.speed_variance &&
        self.start_color == other.start_color &&
        self.end_color == other.end_color &&
        self.start_size == other.start_size &&
        self.end_size == other.end_size &&
        self.gravity == other.gravity &&
        self.particles.len() == other.particles.len()
    }
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            emitting: true,
            max_particles: 100,
            spawn_rate: 10.0,
            spawn_timer: 0.0,
            lifetime: 1.0,
            speed: 50.0,
            speed_variance: 20.0,
            start_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [1.0, 0.0, 0.0, 0.0],
            start_size: 4.0,
            end_size: 1.0,
            gravity: 0.0,
            particles: Vec::new(),
        }
    }
}

/// Audio source component - attach to an entity for positional audio
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioSource {
    /// Path to audio file relative to assets/
    pub path: String,
    /// Whether to loop
    pub looping: bool,
    /// Volume (0.0 - 1.0)
    pub volume: f32,
    /// Whether currently playing
    pub playing: bool,
    /// Whether this is music (background) vs SFX
    pub is_music: bool,
}

impl Default for AudioSource {
    fn default() -> Self {
        Self {
            path: String::new(),
            looping: false,
            volume: 1.0,
            playing: false,
            is_music: false,
        }
    }
}

/// Parent component - entity has a parent (stored as entity index for serialization)
#[derive(Component, Debug, Clone, Copy)]
pub struct Parent(pub Entity);

/// Children component - entity has children (stored as entity indices for serialization)
#[derive(Component, Debug, Clone)]
pub struct Children(pub Vec<Entity>);

impl Default for Children {
    fn default() -> Self {
        Self(Vec::new())
    }
}