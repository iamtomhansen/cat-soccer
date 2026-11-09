//! A procedural soccer ball that rolls around the arena and can be kicked by the cat.

mod effects;
mod mesh;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::physics::Kickable;
use effects::BallEffectsPlugin;
use mesh::ball_mesh;

/// Radius of the ball in world units.
pub const BALL_RADIUS: f32 = 0.06;
/// Where the ball starts, on the ground.
const BALL_START_POS: Vec2 = Vec2::new(-0.5, -0.5);
/// Surface roughness of the ball.
const BALL_ROUGHNESS: f32 = 0.6;
/// Mass of the ball, in kilograms.
const BALL_MASS: f32 = 0.4;
/// Bounciness of the ball, from 0 (dead) to 1 (perfectly elastic). Wins over what it hits.
const BALL_RESTITUTION: f32 = 0.6;
/// Grip of the ball on the ground and walls.
const BALL_FRICTION: f32 = 0.6;
/// Linear damping, so a rolling ball slows down and stops.
const BALL_LINEAR_DAMPING: f32 = 0.4;
/// Angular damping, so the ball stops spinning once it stops rolling.
const BALL_ANGULAR_DAMPING: f32 = 0.8;
/// Ball speed per unit of the kicker's approach speed.
const BALL_KICK_GAIN: f32 = 1.6;

/// Plugin that spawns the soccer ball and its effects.
pub struct BallPlugin;

impl Plugin for BallPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BallEffectsPlugin)
            .add_systems(Startup, spawn_ball);
    }
}

/// Marker for the soccer ball.
#[derive(Component)]
pub struct Ball;

/// Spawns the ball as a dynamic rigid body resting on the ground.
fn spawn_ball(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Ball,
        Name::new("Ball"),
        Mesh3d(meshes.add(ball_mesh(BALL_RADIUS))),
        MeshMaterial3d(materials.add(StandardMaterial {
            perceptual_roughness: BALL_ROUGHNESS,
            ..default()
        })),
        Transform::from_xyz(BALL_START_POS.x, BALL_RADIUS, BALL_START_POS.y),
        RigidBody::Dynamic,
        Collider::sphere(BALL_RADIUS),
        Mass(BALL_MASS),
        Restitution::new(BALL_RESTITUTION).with_combine_rule(CoefficientCombine::Max),
        Friction::new(BALL_FRICTION),
        LinearDamping(BALL_LINEAR_DAMPING),
        AngularDamping(BALL_ANGULAR_DAMPING),
        Kickable { gain: BALL_KICK_GAIN },
    ));
}