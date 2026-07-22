//! ECS component definitions using bevy_ecs.
//!
//! All gameplay data is stored in components attached to entities.
//! Systems query for components to implement behavior.

mod components;
mod resources;
mod systems;

pub use components::*;
pub use resources::*;
pub use systems::*;

use bevy_ecs::schedule::Schedule;

/// Create the core ECS schedule with built-in systems
#[allow(unused_mut)]
pub fn build_core_schedule() -> Schedule {
    let mut schedule = Schedule::default();

    // Core systems are added here as they are implemented
    // Systems are added in order of execution priority
    schedule.add_systems((
        camera_system,
        hierarchy_system,
        movement_system,
        animation_system,
        particle_system,
    ));

    schedule
}