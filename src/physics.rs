//! Physics setup on top of Avian: plugin wiring, wall helpers and model-fitted colliders.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::model::ModelBounds;

/// Debug flag: Whether Avian draws collider outlines.
const DRAW_COLLIDERS: bool = false;

/// Plugin that adds Avian physics and fits colliders to models that ask for one.
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default())
            .add_systems(Update, fit_box_colliders);
        if DRAW_COLLIDERS {
            app.add_plugins(PhysicsDebugPlugin);
        }
    }
}

/// Gives the entity an invisible box collider matching its `ModelBounds` once the model loads.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct FitBoxCollider;

/// Inserts a box collider fitted to the model bounds of each `FitBoxCollider` entity.
fn fit_box_colliders(
    mut commands: Commands,
    models: Query<(Entity, &ModelBounds), (With<FitBoxCollider>, Without<Collider>)>,
) {
    for (entity, bounds) in &models {
        let size = bounds.half_extents * 2.0;
        let shape = Collider::cuboid(size.x, size.y, size.z);
        commands
            .entity(entity)
            .insert(Collider::compound(vec![(bounds.center, Quat::IDENTITY, shape)]));
    }
}

/// Invisible static box walls enclosing `half_extents` (x = world X, y = world Z) around `center`.
pub fn box_walls(
    center: Vec3,
    half_extents: Vec2,
    thickness: f32,
    height: f32,
) -> [(Name, RigidBody, Collider, Transform); 4] {
    let half_t = thickness * 0.5;
    let wall = |size: Vec3, offset: Vec3| {
        (
            Name::new("Wall"),
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
            Transform::from_translation(center + offset + Vec3::Y * height * 0.5),
        )
    };
    // Walls along X are long enough to cover the corners.
    let x_wall = Vec3::new(thickness, height, half_extents.y * 2.0 + thickness * 2.0);
    let z_wall = Vec3::new(half_extents.x * 2.0, height, thickness);
    [
        wall(x_wall, Vec3::X * (half_extents.x + half_t)),
        wall(x_wall, Vec3::NEG_X * (half_extents.x + half_t)),
        wall(z_wall, Vec3::Z * (half_extents.y + half_t)),
        wall(z_wall, Vec3::NEG_Z * (half_extents.y + half_t)),
    ]
}
