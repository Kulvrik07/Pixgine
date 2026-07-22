use bevy_ecs::prelude::*;
use crate::ecs::*;
use crate::scene::scene::{EntityDescriptor, SceneDescriptor, Scene};
use anyhow::Result;

/// Spawn all entities from a scene descriptor into the ECS world
pub fn spawn_scene(world: &mut World, descriptor: SceneDescriptor) -> Result<Scene> {
    let mut scene = Scene::new(&descriptor.name);

    // Pass 1: spawn all entities with their components
    for entity_desc in &descriptor.entities {
        let mut entity = world.spawn_empty();

        // Add components based on serialized data
        for (comp_name, comp_data) in &entity_desc.components {
            match comp_name.as_str() {
                "Transform" => {
                    if let Ok(transform) = serde_json::from_value::<Transform>(comp_data.clone()) {
                        entity.insert(transform);
                    }
                }
                "Sprite" => {
                    if let Ok(sprite) = serde_json::from_value::<Sprite>(comp_data.clone()) {
                        entity.insert(sprite);
                    }
                }
                "Animation" => {
                    if let Ok(anim) = serde_json::from_value::<Animation>(comp_data.clone()) {
                        entity.insert(anim);
                    }
                }
                "Physics" => {
                    if let Ok(physics) = serde_json::from_value::<Physics>(comp_data.clone()) {
                        entity.insert(physics);
                    }
                }
                "Velocity" => {
                    if let Ok(vel) = serde_json::from_value::<Velocity>(comp_data.clone()) {
                        entity.insert(vel);
                    }
                }
                "Player" => {
                    entity.insert(Player);
                }
                "CameraTag" => {
                    entity.insert(CameraTag);
                }
                "ParticleEmitter" => {
                    if let Ok(emitter) = serde_json::from_value::<ParticleEmitter>(comp_data.clone()) {
                        entity.insert(emitter);
                    }
                }
                "AudioSource" => {
                    if let Ok(audio) = serde_json::from_value::<AudioSource>(comp_data.clone()) {
                        entity.insert(audio);
                    }
                }
                "Script" => {
                    if let Some(path) = comp_data.get("path").and_then(|v| v.as_str()) {
                        let source = comp_data.get("source").and_then(|v| v.as_str()).unwrap_or("");
                        entity.insert(Script { path: path.to_string(), source: source.to_string() });
                    }
                }
                _ => {
                    log::warn!("Unknown component type: {}", comp_name);
                }
            }
        }

        scene.entity_map.insert(entity_desc.name.clone(), entity.id());
    }

    // Pass 2: wire up parent/child relationships
    for entity_desc in &descriptor.entities {
        if let Some(comp_data) = entity_desc.components.get("Parent") {
            if let Some(parent_name) = comp_data.get("parent_name").and_then(|v| v.as_str()) {
                if let Some(&parent_entity) = scene.entity_map.get(parent_name) {
                    if let Some(&child_entity) = scene.entity_map.get(&entity_desc.name) {
                        // Set Parent component on child
                        world.entity_mut(child_entity).insert(Parent(parent_entity));
                        // Add to parent's Children
                        if let Some(mut children) = world.get_mut::<Children>(parent_entity) {
                            children.0.push(child_entity);
                        } else {
                            world.entity_mut(parent_entity).insert(Children(vec![child_entity]));
                        }
                    }
                }
            }
        }
    }

    scene.entities = descriptor.entities;
    Ok(scene)
}

/// Serialize all entities from the ECS world into a scene descriptor
pub fn serialize_world(world: &mut World) -> SceneDescriptor {
    let entity_ids: Vec<Entity> = world.query::<Entity>().iter(world).collect();
    let mut name_counter = 0;

    let mut entities = Vec::new();

    for entity in &entity_ids {
        let transform = world.get::<Transform>(*entity);
        let sprite = world.get::<Sprite>(*entity);
        let animation = world.get::<Animation>(*entity);
        let physics = world.get::<Physics>(*entity);
        let velocity = world.get::<Velocity>(*entity);
        let player = world.get::<Player>(*entity);
        let camera_tag = world.get::<CameraTag>(*entity);
        let particle = world.get::<ParticleEmitter>(*entity);
        let audio = world.get::<AudioSource>(*entity);
        let script = world.get::<Script>(*entity);
        let parent = world.get::<Parent>(*entity);
        let _children = world.get::<Children>(*entity);

        let mut components: std::collections::HashMap<String, serde_json::Value> =
            std::collections::HashMap::new();

        if let Some(t) = transform {
            components.insert("Transform".into(), serde_json::to_value(t).unwrap());
        }
        if let Some(s) = sprite {
            components.insert("Sprite".into(), serde_json::to_value(s).unwrap());
        }
        if let Some(a) = animation {
            components.insert("Animation".into(), serde_json::to_value(a).unwrap());
        }
        if let Some(p) = physics {
            components.insert("Physics".into(), serde_json::to_value(p).unwrap());
        }
        if let Some(v) = velocity {
            components.insert("Velocity".into(), serde_json::to_value(v).unwrap());
        }
        if player.is_some() {
            components.insert("Player".into(), serde_json::json!({}));
        }
        if camera_tag.is_some() {
            components.insert("CameraTag".into(), serde_json::json!({}));
        }
        if let Some(p) = particle {
            components.insert("ParticleEmitter".into(), serde_json::to_value(p).unwrap());
        }
        if let Some(a) = audio {
            components.insert("AudioSource".into(), serde_json::to_value(a).unwrap());
        }
        if let Some(s) = script {
            components.insert("Script".into(), serde_json::json!({
                "path": s.path,
                "source": s.source,
            }));
        }
        if let Some(p) = parent {
            // Store parent as a reference name; we need to find the entity name
            if let Some(name) = find_entity_name(p.0, &entity_ids, world) {
                components.insert("Parent".into(), serde_json::json!({
                    "parent_name": name,
                }));
            }
        }

        let name = if let Some(name) = find_first_name(*entity, &entity_ids, world) {
            name
        } else {
            let n = format!("Entity_{}", name_counter);
            name_counter += 1;
            n
        };

        entities.push(EntityDescriptor {
            name,
            components,
        });
    }

    SceneDescriptor {
        name: "scene".to_string(),
        entities,
        tilemap: Default::default(),
    }
}

/// Try to find a human-readable name for an entity by looking at known naming conventions
fn find_first_name(_entity: Entity, _all_entities: &[Entity], _world: &World) -> Option<String> {
    // Since we don't have entity names stored in components, use entity IDs
    // The editor tracks names via VP's entities list - we don't persist them yet
    None
}

/// Find the name of an entity from the serialized entity descriptors
fn find_entity_name(entity: Entity, entity_ids: &[Entity], _world: &World) -> Option<String> {
    // Find the index of this entity in the serialization order
    entity_ids.iter().position(|e| *e == entity).map(|i| format!("Entity_{}", i))
}