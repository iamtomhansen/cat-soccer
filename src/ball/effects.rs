//! Visual flair for the ball: a translucent flash where it is kicked.

use bevy::ecs::entity::EntityHashMap;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use super::{Ball, BALL_RADIUS};
use crate::physics::Kicked;

/// How long a kick flash takes to fade away, in seconds.
const FLASH_SECONDS: f32 = 0.25;
/// Flash radius when it appears, as a fraction of the ball radius.
const FLASH_START_SCALE: f32 = 0.3;
/// Flash radius when it has faded, as a fraction of the ball radius.
const FLASH_END_SCALE: f32 = 1.4;
/// Colour of the flash.
const FLASH_COLOR: Color = Color::srgb(1.0, 0.95, 0.8);
/// Opacity of the flash from the hardest kick.
const FLASH_MAX_ALPHA: f32 = 0.7;
/// Opacity of the flash from the softest kick.
const FLASH_MIN_ALPHA: f32 = 0.15;
/// Kick speed gain, in m/s, that gives a flash of full opacity.
const FLASH_FULL_SPEED_GAIN: f32 = 1.5;
/// Shortest time between flashes on the same ball, so a ball held against a wall or dribbled
/// (which is kicked every frame) doesn't fill the screen with flashes.
const FLASH_COOLDOWN_SECONDS: f32 = 0.2;

/// Plugin for the ball's visual effects.
pub struct BallEffectsPlugin;

impl Plugin for BallEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_flash_mesh)
            .add_systems(Update, (spawn_kick_flashes, fade_kick_flashes));
    }
}

/// Mesh shared by all kick flashes: a unit sphere scaled per flash.
#[derive(Resource)]
struct FlashMesh(Handle<Mesh>);

/// A fading sphere left where the ball was kicked.
#[derive(Component)]
struct KickFlash {
    /// Time since the flash appeared.
    timer: Timer,
    /// Opacity the flash started at.
    alpha: f32,
}

/// Creates the shared flash mesh.
fn setup_flash_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(FlashMesh(meshes.add(Sphere::new(1.0).mesh().uv(16, 12))));
}

/// Spawns a flash at the point of each ball kick, brighter for harder kicks.
fn spawn_kick_flashes(
    mut commands: Commands,
    mut kicks: MessageReader<Kicked>,
    time: Res<Time>,
    mut last_flash: Local<EntityHashMap<f32>>,
    balls: Query<(), With<Ball>>,
    flash_mesh: Res<FlashMesh>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let now = time.elapsed_secs();
    for kick in kicks.read() {
        if !balls.contains(kick.entity) {
            continue;
        }
        if last_flash.get(&kick.entity).is_some_and(|&t| now - t < FLASH_COOLDOWN_SECONDS) {
            continue;
        }
        last_flash.insert(kick.entity, now);

        let strength = (kick.speed_gain / FLASH_FULL_SPEED_GAIN).clamp(0.0, 1.0);
        let alpha = FLASH_MIN_ALPHA.lerp(FLASH_MAX_ALPHA, strength);
        // Each flash has its own material so it can fade on its own.
        let material = materials.add(StandardMaterial {
            base_color: FLASH_COLOR.with_alpha(alpha),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });
        commands.spawn((
            Name::new("Kick flash"),
            KickFlash { timer: Timer::from_seconds(FLASH_SECONDS, TimerMode::Once), alpha },
            Mesh3d(flash_mesh.0.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(kick.point)
                .with_scale(Vec3::splat(BALL_RADIUS * FLASH_START_SCALE)),
            NotShadowCaster,
        ));
    }
}

/// Grows and fades each kick flash, then removes it.
fn fade_kick_flashes(
    mut commands: Commands,
    time: Res<Time>,
    mut flashes: Query<(Entity, &mut KickFlash, &mut Transform, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut flash, mut transform, material) in &mut flashes {
        flash.timer.tick(time.delta());
        if flash.timer.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }
        let t = flash.timer.fraction();
        // Ease out: grow fast at first, then settle.
        let grow = 1.0 - (1.0 - t) * (1.0 - t);
        transform.scale = Vec3::splat(BALL_RADIUS * FLASH_START_SCALE.lerp(FLASH_END_SCALE, grow));
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.base_color.set_alpha(flash.alpha * (1.0 - t));
        }
    }
}