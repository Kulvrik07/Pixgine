//! Built-in ECS systems

use bevy_ecs::prelude::*;
use crate::ecs::components::*;
use crate::ecs::resources::TimeResource;
use rand::Rng;

/// Update velocity-based movement each frame
pub fn movement_system(mut query: Query<(&Velocity, &mut Transform)>, time: Res<TimeResource>) {
    for (velocity, mut transform) in query.iter_mut() {
        transform.x += velocity.x * time.delta;
        transform.y += velocity.y * time.delta;
    }
}

/// Update animation frames
pub fn animation_system(mut query: Query<&mut Animation>, time: Res<TimeResource>) {
    for mut anim in query.iter_mut() {
        if !anim.playing {
            continue;
        }

        anim.elapsed += time.delta;
        if anim.elapsed >= anim.frame_durations[anim.current_frame] {
            anim.elapsed = 0.0;
            anim.current_frame += 1;

            if anim.current_frame >= anim.frame_durations.len() {
                if anim.looping {
                    anim.current_frame = 0;
                } else {
                    anim.current_frame = anim.frame_durations.len() - 1;
                    anim.playing = false;
                }
            }
        }
    }
}

/// Particle system - update all ParticleEmitter components
pub fn particle_system(mut query: Query<(&mut ParticleEmitter, &mut Transform)>, time: Res<TimeResource>) {
    let dt = time.delta;
    let mut rng = rand::thread_rng();
    for (mut emitter, transform) in query.iter_mut() {
        if !emitter.emitting { continue; }

        // Spawn new particles
        emitter.spawn_timer += dt;
        let spawn_interval = 1.0 / emitter.spawn_rate.max(0.001);
        while emitter.spawn_timer >= spawn_interval && emitter.particles.len() < emitter.max_particles {
            emitter.spawn_timer -= spawn_interval;
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let speed = emitter.speed + rng.gen_range(-emitter.speed_variance..emitter.speed_variance);
            let p = Particle {
                x: transform.x,
                y: transform.y,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed,
                lifetime: emitter.lifetime,
                age: 0.0,
                size: emitter.start_size,
                color: emitter.start_color,
            };
            emitter.particles.push(p);
        }

        // Update existing particles
        let gravity = emitter.gravity;
        let start_col = emitter.start_color;
        let end_col = emitter.end_color;
        let start_sz = emitter.start_size;
        let end_sz = emitter.end_size;
        emitter.particles.retain_mut(|p| {
            p.age += dt;
            if p.age >= p.lifetime { return false; }
            let t = p.age / p.lifetime;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vy += gravity * dt;
            p.size = start_sz + (end_sz - start_sz) * t;
            for i in 0..4 {
                p.color[i] = start_col[i] + (end_col[i] - start_col[i]) * t;
            }
            true
        });
    }
}

/// Hierarchy system - propagate parent transforms to children
pub fn hierarchy_system(
    mut query: ParamSet<(
        Query<(Entity, &Parent, &Transform)>,  // read-only: children with parents
        Query<(Entity, &Transform)>,            // read-only: all transforms with entity
        Query<(Entity, &mut Transform)>,       // write: apply world transforms
    )>,
    _time: Res<TimeResource>,
) {
    // Collect all parent transforms
    let parent_transforms: std::collections::HashMap<Entity, (f32, f32, f32)> = {
        let q = query.p1();
        q.iter().map(|(e, t)| (e, (t.x, t.y, t.rotation))).collect()
    };
    
    // Collect children + parent relationships
    let children: Vec<(Entity, Entity, f32, f32, f32)> = {
        let q = query.p0();
        q.iter().map(|(e, p, t)| (e, p.0, t.x, t.y, t.rotation)).collect()
    };
    
    // Compute world transforms bottom-up
    let mut world_transforms: std::collections::HashMap<Entity, (f32, f32, f32)> = std::collections::HashMap::new();
    for _ in 0..64 {
        let mut updated = false;
        for &(entity, parent_entity, lx, ly, lrot) in &children {
            if world_transforms.contains_key(&entity) { continue; }
            if let Some(&(px, py, prot)) = parent_transforms.get(&parent_entity).or_else(|| world_transforms.get(&parent_entity)) {
                let cos = prot.cos();
                let sin = prot.sin();
                let wx = px + lx * cos - ly * sin;
                let wy = py + lx * sin + ly * cos;
                world_transforms.insert(entity, (wx, wy, lrot));
                updated = true;
            }
        }
        if !updated { break; }
    }
    
    // Apply world transforms
    let mut q = query.p2();
    for (entity, mut transform) in q.iter_mut() {
        if let Some(&(wx, wy, _)) = world_transforms.get(&entity) {
            transform.x = wx;
            transform.y = wy;
        }
    }
}

/// Camera system - find entity with CameraTag and update view
pub fn camera_system(
    query: Query<(&Transform, &CameraTag)>,
    view_state: Option<ResMut<crate::ecs::resources::ViewState>>,
) {
    if let Some(mut vs) = view_state {
        for (transform, _) in query.iter() {
            vs.camera_x = transform.x;
            vs.camera_y = transform.y;
        }
    }
}