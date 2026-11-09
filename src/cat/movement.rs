//! Moves the cat from analog joystick input and tracks its movement state.

use bevy::prelude::*;
use touchy::prelude::*;
use avian3d::prelude::*;
use super::Cat;
use crate::physics::{GameLayer, MoveHit};

/// Top speed of the cat, reached at full joystick deflection.
pub const CAT_SPEED: f32 = 1.0;
/// Joystick magnitudes below this are treated as no input.
const STICK_DEADZONE: f32 = 0.05;
/// Seconds the cat takes to go from standstill to `CAT_SPEED`.
const ACCELERATION_SECONDS: f32 = 0.2;
/// How quickly the cat turns to face its heading.
const TURN_RATE: f32 = 12.0;
/// Intended speeds below this don't turn the cat, so it keeps its heading as it stops.
const MIN_TURN_SPEED: f32 = 0.01;
/// Fraction of `CAT_SPEED` above which an idle cat starts walking.
const WALK_START_FRACTION: f32 = 0.04;
/// Fraction of `CAT_SPEED` below which a walking cat goes idle.
const WALK_STOP_FRACTION: f32 = 0.02;

/// The plugin that sets up and runs the cat movement.
pub struct CatMovementPlugin;

impl Plugin for CatMovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, move_cat);
    }
}

/// Whether the cat is standing still or walking.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MoveState {
    /// Standing still.
    #[default]
    Idle,
    /// Moving across the ground.
    Walking,
}

/// The cat's current movement, read by animation and other systems.
#[derive(Component, Debug, Default)]
pub struct CatMotion {
    /// Velocity the cat is trying to move at, accelerating towards the joystick, before collisions.
    pub intended_velocity: Vec3,
    /// Velocity the cat actually moved at last frame, after sliding along walls.
    pub velocity: Vec3,
    /// Current movement state.
    pub state: MoveState,
}

impl CatMotion {
    /// Current ground speed.
    pub fn speed(&self) -> f32 {
        self.velocity.length()
    }
}

/// Move the cat at a speed proportional to joystick deflection.
pub fn move_cat(
    time: Res<Time>,
    sticks: Query<&JoystickValue>,
    mut cats: Query<(Entity, &mut Transform, &mut CatMotion, Option<&Collider>),
        With<Cat>>,
    move_and_slide: MoveAndSlide,
    mut hits: MessageWriter<MoveHit>,
) {
    let stick = sticks.single().map(|s| s.0.clamp_length_max(1.0)).unwrap_or(Vec2::ZERO);
    let stick = if stick.length() < STICK_DEADZONE { Vec2::ZERO } else { stick };

    // Convert 2D screen joystick (X = right/left, Y = up/down)
    // to 3D world space (X = X, Y = 0, Z = -Y)
    let target_velocity = Vec3::new(stick.x, 0.0, -stick.y) * CAT_SPEED;
    let max_delta_v = CAT_SPEED / ACCELERATION_SECONDS * time.delta_secs();

    for (entity, mut transform, mut motion, collider) in &mut cats {
        let delta = (target_velocity - motion.intended_velocity).clamp_length_max(max_delta_v);
        motion.intended_velocity += delta;

        // Turn towards the input before moving, so move-and-slide's depenetration pass pushes a
        // turning collider out of walls in the same frame. Facing the input (not the measured
        // velocity) keeps those pushes from feeding back into the heading, which made it jitter.
        if motion.intended_velocity.length() > MIN_TURN_SPEED {
            let target_rot =
                Transform::default().looking_to(motion.intended_velocity, Vec3::Y).rotation;
            transform.rotation = transform.rotation.slerp(target_rot, TURN_RATE * time.delta_secs());
        }

        let start = transform.translation;

        // Slide along anything in the way.
        if let Some(collider) = collider {
            // The cat walks on the ground plane itself, so it only collides with things on it.
            let filter = SpatialQueryFilter::from_mask(GameLayer::Default)
                .with_excluded_entities([entity]);
            let intended_velocity = motion.intended_velocity;
            let out = move_and_slide.move_and_slide(
                collider,
                transform.translation,
                transform.rotation,
                motion.intended_velocity,
                time.delta(),
                &MoveAndSlideConfig::default(),
                &filter,
                |hit| {
                    hits.write(MoveHit {
                        other: hit.entity,
                        point: hit.point,
                        normal: hit.normal.as_vec3(),
                        velocity: intended_velocity,
                    });
                    MoveAndSlideHitResponse::Accept
                },
            );
            transform.translation = out.position.with_y(start.y);
        } else {
            transform.translation += motion.intended_velocity * time.delta_secs();
        }

        // Measure what really happened, so animation follows the slide, not the input.
        // (Move-and-slide's projected velocity flickers between frames at a wall.)
        if time.delta_secs() > 0.0 {
            motion.velocity = (transform.translation - start).with_y(0.0) / time.delta_secs();
        }

        let speed = motion.speed();
        motion.state = match motion.state {
            MoveState::Idle if speed > WALK_START_FRACTION * CAT_SPEED => MoveState::Walking,
            MoveState::Walking if speed < WALK_STOP_FRACTION * CAT_SPEED => MoveState::Idle,
            state => state,
        };
    }
}
