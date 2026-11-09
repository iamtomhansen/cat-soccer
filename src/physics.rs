//! Physics setup on top of Avian: plugin wiring, collision layers, wall and floor helpers,
//! model-fitted colliders, and kicking things a character bumps into.

use avian3d::prelude::*;
use avian3d::dynamics::solver::SolverConfig;
use bevy::prelude::*;

use crate::model::ModelBounds;

/// Debug flag: Whether Avian draws collider outlines.
const DRAW_COLLIDERS: bool = false;
/// Impact speed below which bodies stop bouncing. Avian default is 1 m/s.
/// A lower value lets the ball bounce off walls.
const RESTITUTION_THRESHOLD: f32 = 0.1;

/// Plugin that adds Avian physics and fits colliders to models that ask for one.
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default())
            .insert_resource(SolverConfig {
                restitution_threshold: RESTITUTION_THRESHOLD,
                ..default()
            })
            .add_message::<MoveHit>()
            .add_message::<Kicked>()
            .add_systems(Update, (fit_colliders, kick_on_hit));
        if DRAW_COLLIDERS {
            app.add_plugins(PhysicsDebugPlugin);
        }
    }
}

/// Collision layers used by the game.
#[derive(PhysicsLayer, Clone, Copy, Debug, Default)]
pub enum GameLayer {
    /// Everything that isn't on a more specific layer.
    #[default]
    Default,
    /// The ground. Characters moved with move-and-slide don't collide with it.
    Ground,
}

/// Sent when a character moved with move-and-slide touches another collider.
#[derive(Message, Clone, Copy, Debug)]
pub struct MoveHit {
    /// The collider it touched.
    pub other: Entity,
    /// Where the two touched, in world space.
    pub point: Vec3,
    /// Outward surface normal of `other` at the touch point, pointing back at the mover.
    pub normal: Vec3,
    /// Velocity the mover was trying to move at.
    pub velocity: Vec3,
}

/// A dynamic body that is knocked away when a character walks into it.
#[derive(Component, Clone, Copy, Debug)]
pub struct Kickable {
    /// Speed given to the body along the kick, per unit of the character's approach speed.
    pub gain: f32,
}

/// Sent when a `Kickable` body is kicked, for effects and sounds to react to.
#[derive(Message, Clone, Copy, Debug)]
pub struct Kicked {
    /// The body that was kicked.
    pub entity: Entity,
    /// Where it was kicked, in world space.
    pub point: Vec3,
    /// How much the kick sped the body up.
    pub speed_gain: f32,
}

/// Gives the entity an invisible collider fitted to its `ModelBounds` once the model loads.
///
/// An enum so more fitted shapes (e.g. sphere, capsule) can be added alongside the box.
#[derive(Component, Clone, Copy, Debug, Default)]
pub enum FitCollider {
    /// Box matching the bounds that turns with the entity.
    #[default]
    Box,
}

/// Inserts a collider fitted to the model bounds of each `FitCollider` entity.
fn fit_colliders(
    mut commands: Commands,
    models: Query<(Entity, &ModelBounds, &FitCollider), Without<Collider>>,
) {
    for (entity, bounds, fit) in &models {
        let half = bounds.half_extents;
        let shape = match fit {
            FitCollider::Box => Collider::cuboid(half.x * 2.0, half.y * 2.0, half.z * 2.0),
        };
        commands
            .entity(entity)
            .insert(Collider::compound(vec![(bounds.center, Quat::IDENTITY, shape)]));
    }
}

/// Gives `Kickable` bodies that a character walks into at least the kick velocity.
///
/// The character stops at the body's surface, so the kick is a velocity change rather than a push:
/// it doesn't depend on mass and can't squeeze the body through a wall.
fn kick_on_hit(
    mut hits: MessageReader<MoveHit>,
    mut bodies: Query<(&Kickable, &mut LinearVelocity)>,
    mut kicks: MessageWriter<Kicked>,
) {
    for hit in hits.read() {
        let Ok((kick, mut velocity)) = bodies.get_mut(hit.other) else { continue; };
        let Some(dir) = (-hit.normal).with_y(0.0).try_normalize() else { continue; };
        let approach = hit.velocity.dot(dir);
        if approach <= 0.0 {
            continue;
        }
        // Top up rather than add, so a body touched on many frames isn't kicked again each time.
        let along = velocity.0.dot(dir);
        let target = approach * kick.gain;
        if along < target {
            velocity.0 += dir * (target - along);
            kicks.write(Kicked { entity: hit.other, point: hit.point, speed_gain: target - along });
        }
    }
}

/// Static ground plane at height `y`, on the `Ground` layer.
pub fn ground(y: f32) -> (Name, RigidBody, Collider, CollisionLayers, Transform) {
    (
        Name::new("Ground"),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
        CollisionLayers::new(GameLayer::Ground, LayerMask::ALL),
        Transform::from_xyz(0.0, y, 0.0),
    )
}

/// A single invisible static wall whose inner face runs along the ground from `from` to `to`.
///
/// The wall's thickness extends to the right of `from -> to` (seen from above), and it is
/// lengthened by `thickness` at both ends so neighbouring walls overlap at outward corners.
pub fn wall(from: Vec2, to: Vec2, thickness: f32, height: f32) -> (Name, RigidBody, Collider, Transform) {
    let along = to - from;
    let dir = along.normalize_or(Vec2::X);
    let outward = Vec2::new(-dir.y, dir.x);
    let mid = (from + to) * 0.5 + outward * thickness * 0.5;
    (
        Name::new("Wall"),
        RigidBody::Static,
        Collider::cuboid(along.length() + thickness * 2.0, height, thickness),
        Transform::from_xyz(mid.x, height * 0.5, mid.y)
            .with_rotation(Quat::from_rotation_y(-dir.y.atan2(dir.x))),
    )
}

/// Invisible static walls enclosing `half_extents` (x = world X, y = world Z) around `center`,
/// with each corner cut off by a 45-degree wall `bevel` long on each side.
///
/// Bevels stop round things getting wedged in the corners. A `bevel` of 0 gives square corners.
pub fn box_walls(
    center: Vec2,
    half_extents: Vec2,
    bevel: f32,
    thickness: f32,
    height: f32,
) -> Vec<(Name, RigidBody, Collider, Transform)> {
    let (hx, hz) = (half_extents.x, half_extents.y);
    let b = bevel.clamp(0.0, hx.min(hz));
    // Octagon corners, going clockwise seen from above so each wall's right side faces out.
    let corners = [
        Vec2::new(-hx + b, -hz),
        Vec2::new(-hx, -hz + b),
        Vec2::new(-hx, hz - b),
        Vec2::new(-hx + b, hz),
        Vec2::new(hx - b, hz),
        Vec2::new(hx, hz - b),
        Vec2::new(hx, -hz + b),
        Vec2::new(hx - b, -hz),
    ];
    (0..corners.len())
        .map(|i| (corners[i], corners[(i + 1) % corners.len()]))
        .filter(|(from, to)| from.distance(*to) > f32::EPSILON)
        .map(|(from, to)| wall(center + from, center + to, thickness, height))
        .collect()
}