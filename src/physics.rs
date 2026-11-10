//! Physics setup on top of Avian: plugin wiring, collision layers, wall and floor helpers,
//! model-fitted colliders, and kicking things a character bumps into.

use avian3d::prelude::*;
use avian3d::dynamics::solver::SolverConfig;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;

use crate::model::ModelBounds;
use crate::rng::Rng;

/// Debug flag: Whether Avian draws collider outlines.
const DRAW_COLLIDERS: bool = false;
/// Impact speed below which bodies stop bouncing. Avian default is 1 m/s.
/// A lower value lets the ball bounce off walls.
const RESTITUTION_THRESHOLD: f32 = 0.1;
/// Stops Avian from recycling contact points, which can cause infinite bounce/jitter.
const CONTACT_RECYCLE_DISTANCE: f32 = 0.0;
/// Smallest kick, as a speed gain in m/s, that lifts the body; gentler touches (dribbling) stay low.
const LIFT_GAIN_THRESHOLD: f32 = 0.5;
/// Shortest time between kicks that lift the same body, stopping infinite lift when pinned.
const KICK_LIFT_COOLDOWN_SECONDS: f32 = 0.4;

/// Plugin that adds Avian physics and fits colliders to models that ask for one.
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        // Physics steps at a fixed 64 Hz, so without interpolation bodies visibly stutter on any
        // other refresh rate. Interpolating draws them smoothly in between steps.
        app.add_plugins(PhysicsPlugins::default().set(PhysicsInterpolationPlugin::interpolate_all()))
            .insert_resource(SolverConfig {
                restitution_threshold: RESTITUTION_THRESHOLD,
                ..default()
            })
            .insert_resource(NarrowPhaseConfig {
                recycle_distance: CONTACT_RECYCLE_DISTANCE,
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
    /// Least upward speed given by a kick, per unit of kick speed.
    pub lift_min: f32,
    /// Most upward speed given by a kick, per unit of kick speed.
    pub lift_max: f32,
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

/// Gives `Kickable` bodies that a character walks into at least the kick velocity, and pops
/// them up by a random lift on a firm kick.
///
/// The character stops at the body's surface, so the kick is a velocity change rather than a push:
/// it doesn't depend on mass and can't squeeze the body through a wall.
fn kick_on_hit(
    mut hits: MessageReader<MoveHit>,
    mut bodies: Query<(&Kickable, &mut LinearVelocity)>,
    mut kicks: MessageWriter<Kicked>,
    time: Res<Time>,
    mut rng: ResMut<Rng>,
    mut last_lift: Local<EntityHashMap<f32>>,
) {
    let now = time.elapsed_secs();
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
            let speed_gain = target - along;
            velocity.0 += dir * speed_gain;
            kicks.write(Kicked { entity: hit.other, point: hit.point, speed_gain });

            // Lift only on strong kicks.
            let cooling = last_lift.get(&hit.other).is_some_and(|&t| now - t < KICK_LIFT_COOLDOWN_SECONDS);
            if speed_gain >= LIFT_GAIN_THRESHOLD && !cooling {
                velocity.0.y = target * rng.range_f32(kick.lift_min, kick.lift_max);
                last_lift.insert(hit.other, now);
            }
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

/// Corners of a box of `half_extents` (x = world X, y = world Z) around `center`, seen from above,
/// with each corner cut off by a 45-degree edge `bevel` long on each side.
///
/// Corners go clockwise seen from above, the winding `outline_walls` expects. Bevels stop round
/// things getting wedged in the corners; a `bevel` of 0 gives square corners.
pub fn box_outline(center: Vec2, half_extents: Vec2, bevel: f32) -> Vec<Vec2> {
    let (hx, hz) = (half_extents.x, half_extents.y);
    let b = bevel.clamp(0.0, hx.min(hz));
    [
        Vec2::new(-hx + b, -hz),
        Vec2::new(-hx, -hz + b),
        Vec2::new(-hx, hz - b),
        Vec2::new(-hx + b, hz),
        Vec2::new(hx - b, hz),
        Vec2::new(hx, hz - b),
        Vec2::new(hx, -hz + b),
        Vec2::new(hx - b, -hz),
    ]
    .into_iter()
    .map(|corner| center + corner)
    .collect()
}

/// Invisible static walls whose inner faces run along a closed, clockwise (seen from above) outline.
pub fn outline_walls(
    outline: &[Vec2],
    thickness: f32,
    height: f32,
) -> Vec<(Name, RigidBody, Collider, Transform)> {
    (0..outline.len())
        .map(|i| (outline[i], outline[(i + 1) % outline.len()]))
        .filter(|(from, to)| from.distance(*to) > f32::EPSILON)
        .map(|(from, to)| wall(from, to, thickness, height))
        .collect()
}
