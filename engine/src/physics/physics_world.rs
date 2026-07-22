use bevy_ecs::entity::Entity;
use bevy_ecs::system::Query;
use rapier2d::prelude::*;
use crate::ecs::*;

/// Wraps rapier2d physics world and integrates with ECS
pub struct PhysicsWorld {
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub gravity: Vector<f32>,
    pub integration_parameters: IntegrationParameters,
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: BroadPhaseMultiSap,
    pub narrow_phase: NarrowPhase,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub query_pipeline: QueryPipeline,
    pub ccd_solver: CCDSolver,
    pub entity_map: std::collections::HashMap<Entity, RigidBodyHandle>,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self {
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            gravity: Vector::new(0.0, 0.0), // Top-down games usually have no gravity
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhaseMultiSap::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            query_pipeline: QueryPipeline::new(),
            ccd_solver: CCDSolver::new(),
            entity_map: std::collections::HashMap::new(),
        }
    }

    /// Create a rapier rigid body for an entity with a Physics component
    pub fn create_body(&mut self, entity: Entity, transform: &Transform, physics: &Physics) {
        let body_type = match physics.body_type {
            PhysicsBodyType::Static => RigidBodyType::Fixed,
            PhysicsBodyType::Dynamic => RigidBodyType::Dynamic,
            PhysicsBodyType::Kinematic => RigidBodyType::KinematicPositionBased,
        };

        let body = RigidBodyBuilder::new(body_type)
            .translation(vector![transform.x, transform.y])
            .build();
        let handle = self.rigid_body_set.insert(body);
        self.entity_map.insert(entity, handle);

        // Create collider
        let collider = ColliderBuilder::cuboid(
            physics.collider_width / 2.0,
            physics.collider_height / 2.0,
        )
        .friction(physics.friction)
        .restitution(physics.restitution)
        .sensor(physics.is_trigger)
        .build();
        self.collider_set.insert_with_parent(collider, handle, &mut self.rigid_body_set);
    }

    /// Step the physics simulation
    pub fn step(&mut self) {
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &(),
        );
    }

    /// Sync physics bodies back to ECS transforms
    pub fn sync_to_ecs(&self, query: &mut Query<(&Physics, &mut Transform)>) {
        for (_physics, _transform) in query.iter_mut() {
            if _physics.body_type == PhysicsBodyType::Dynamic {
                // Would need entity mapping to find the rigid body
                // TODO: look up rigid body by entity in self.entity_map
            }
        }
    }
}