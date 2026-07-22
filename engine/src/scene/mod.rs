//! Scene system - loading, saving, and serializing ECS worlds.
//!
//! Scenes are stored as JSON files containing entity definitions
//! with their components serialized.

mod scene;
mod serializer;

pub use scene::*;
pub use serializer::*;