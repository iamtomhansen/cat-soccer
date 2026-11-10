//! A soccer ball that rolls around the arena and can be kicked by the cat.
//! The parametric ball is here as a fallback in case the glTF model has issues.

mod effects;
mod mesh;
mod sound;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::model::{ModelConfig, ModelSize};
use crate::physics::Kickable;
use effects::BallEffectsPlugin;
use mesh::ball_mesh;
use sound::BallSoundPlugin;

/// Path to the soccer ball's glTF model.
const BALL_MODEL_PATH: &str = "models/soccerball.glb";
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
/// Least upward speed a strong kick gives the ball, per unit of kick speed.
const BALL_KICK_LIFT_MIN: f32 = 0.2;
/// Most upward speed a strong kick gives the ball, per unit of kick speed.
const BALL_KICK_LIFT_MAX: f32 = 1.0;

/// Plugin that spawns the soccer ball, its effects, and its sounds.
pub struct BallPlugin;

impl Plugin for BallPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            BallEffectsPlugin,
            BallSoundPlugin,
        ))
            .add_systems(Startup, (
                // spawn_ball_parametric,
                spawn_ball_gltf,
            ));
    }
}

/// Marker for the soccer ball.
#[derive(Component)]
pub struct Ball;

/// Spawns the ball as a dynamic rigid body resting on the ground (using Bevy primitives).
fn spawn_ball_parametric(
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
        Kickable {
            gain: BALL_KICK_GAIN,
            lift_min: BALL_KICK_LIFT_MIN,
            lift_max: BALL_KICK_LIFT_MAX,
        },
    ));
}

/// Spawns the ball as a dynamic rigid body resting on the ground (from a glTF model).
fn spawn_ball_gltf(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Ball,
        Name::new("Ball"),
        Transform::from_xyz(BALL_START_POS.x, BALL_RADIUS * 4.0, BALL_START_POS.y),
        RigidBody::Dynamic,
        Collider::sphere(BALL_RADIUS),
        Mass(BALL_MASS),
        Restitution::new(BALL_RESTITUTION).with_combine_rule(CoefficientCombine::Max),
        Friction::new(BALL_FRICTION),
        LinearDamping(BALL_LINEAR_DAMPING),
        AngularDamping(BALL_ANGULAR_DAMPING),
        Kickable {
            gain: BALL_KICK_GAIN,
            lift_min: BALL_KICK_LIFT_MIN,
            lift_max: BALL_KICK_LIFT_MAX,
        },
        ModelConfig::new(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(BALL_MODEL_PATH)),
            ModelSize::Height(BALL_RADIUS * 2.0),
        ),
    ));
}