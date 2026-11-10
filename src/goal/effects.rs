//! Visual flair for the goal: a puff of smoke where it vanishes, and a pop where it reappears.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use super::mesh::goal_bars;
use super::{relocate_goals, GoalMoved, GoalVisual, GOAL_BAR_THICKNESS, GOAL_HEIGHT, GOAL_WIDTH};
use crate::rng::Rng;

/// Puffs of smoke along each bar where the goal vanishes.
const VANISH_PUFFS_PER_BAR: u32 = 8;
/// Puffs of smoke along each bar where the goal reappears.
const APPEAR_PUFFS_PER_BAR: u32 = 3;
/// How long a puff takes to fade away, in seconds.
const PUFF_SECONDS: f32 = 0.75;
/// Puff radius when it appears, in world units.
const PUFF_START_RADIUS: f32 = 0.01;
/// Puff radius when it has faded, in world units.
const PUFF_END_RADIUS: f32 = 0.05;
/// Colour of the smoke.
const PUFF_COLOR: Color = Color::srgba(0.8, 0.95, 0.75, 1.0);
/// Opacity of a puff when it appears.
const PUFF_ALPHA: f32 = 0.8;
/// Fastest a puff drifts away from its bar, in m/s.
const PUFF_MAX_SPREAD_SPEED: f32 = 0.25;
/// Upward drift added to every puff, in m/s.
const PUFF_RISE_SPEED: f32 = 0.15;
/// Fraction of its speed a puff keeps after one second.
const PUFF_DRAG: f32 = 0.05;
/// Seconds the goal takes to grow back to full size where it reappears.
const APPEAR_SECONDS: f32 = 0.35;
/// How far the goal overshoots its size as it grows in (the "back" in an ease-out-back curve).
const APPEAR_OVERSHOOT: f32 = 1.7;

/// Plugin for the goal's visual effects.
pub struct GoalEffectsPlugin;

impl Plugin for GoalEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_puff_mesh)
            .add_systems(Update, (
                start_goal_poofs.after(relocate_goals),
                drift_puffs,
                grow_appearing_goals,
            ));
    }
}

/// Mesh shared by all puffs: a unit sphere scaled per puff.
#[derive(Resource)]
struct PuffMesh(Handle<Mesh>);

/// A drifting, fading ball of smoke.
#[derive(Component)]
struct Puff {
    /// Time since the puff appeared.
    timer: Timer,
    /// Current drift, in m/s.
    velocity: Vec3,
}

/// Grows a goal's visual from nothing to full size.
#[derive(Component)]
struct GoalAppear {
    /// Time since the goal reappeared.
    timer: Timer,
}

/// Creates the shared puff mesh.
fn setup_puff_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(PuffMesh(meshes.add(Sphere::new(1.0).mesh().uv(12, 8))));
}

/// Puffs smoke when a goal disappears or reappears.
fn start_goal_poofs(
    mut commands: Commands,
    mut moves: MessageReader<GoalMoved>,
    visuals: Query<(Entity, &ChildOf), With<GoalVisual>>,
    puff_mesh: Res<PuffMesh>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<Rng>,
) {
    for moved in moves.read() {
        for (transform, count) in [(moved.from, VANISH_PUFFS_PER_BAR), (moved.to, APPEAR_PUFFS_PER_BAR)] {
            for bar in goal_bars(GOAL_WIDTH, GOAL_HEIGHT, GOAL_BAR_THICKNESS) {
                for _ in 0..count {
                    let along = Vec3::new(rng.f32(), rng.f32(), rng.f32()) - Vec3::splat(0.5);
                    let point = transform.transform_point(bar.offset + along * bar.size);
                    let spread = Vec3::new(
                        rng.range_f32(-1.0, 1.0),
                        rng.range_f32(-1.0, 1.0),
                        rng.range_f32(-1.0, 1.0),
                    );
                    let velocity = spread.normalize_or_zero() * rng.range_f32(0.0, PUFF_MAX_SPREAD_SPEED)
                        + Vec3::Y * PUFF_RISE_SPEED;
                    // Each puff has its own material so it can fade on its own.
                    let puff_color = PUFF_COLOR
                        .with_alpha(PUFF_ALPHA)
                        .rotate_hue(rng.range_f32(-45.0, 45.0));
                    let material = materials.add(StandardMaterial {
                        base_color: puff_color,
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..default()
                    });
                    commands.spawn((
                        Name::new("Goal puff"),
                        Puff { timer: Timer::from_seconds(PUFF_SECONDS, TimerMode::Once), velocity },
                        Mesh3d(puff_mesh.0.clone()),
                        MeshMaterial3d(material),
                        Transform::from_translation(point).with_scale(Vec3::splat(PUFF_START_RADIUS)),
                        NotShadowCaster,
                    ));
                }
            }
        }

        for (visual, child_of) in &visuals {
            if child_of.parent() == moved.goal {
                commands.entity(visual).insert((
                    GoalAppear { timer: Timer::from_seconds(APPEAR_SECONDS, TimerMode::Once) },
                    Transform::from_scale(Vec3::ZERO),
                ));
            }
        }
    }
}

/// Moves, grows and fades each puff, then removes it.
fn drift_puffs(
    mut commands: Commands,
    time: Res<Time>,
    mut puffs: Query<(Entity, &mut Puff, &mut Transform, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();
    for (entity, mut puff, mut transform, material) in &mut puffs {
        puff.timer.tick(time.delta());
        if puff.timer.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation += puff.velocity * dt;
        puff.velocity *= PUFF_DRAG.powf(dt);
        let t = puff.timer.fraction();
        // Ease out: billow fast at first, then settle.
        let grow = 1.0 - (1.0 - t) * (1.0 - t);
        transform.scale = Vec3::splat(PUFF_START_RADIUS.lerp(PUFF_END_RADIUS, grow));
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.base_color.set_alpha(PUFF_ALPHA * (1.0 - t));
        }
    }
}

/// Grows each reappearing goal to full size with a little overshoot.
fn grow_appearing_goals(
    mut commands: Commands,
    time: Res<Time>,
    mut goals: Query<(Entity, &mut GoalAppear, &mut Transform)>,
) {
    for (entity, mut appear, mut transform) in &mut goals {
        appear.timer.tick(time.delta());
        if appear.timer.is_finished() {
            transform.scale = Vec3::ONE;
            commands.entity(entity).remove::<GoalAppear>();
            continue;
        }
        // Ease out back: shoot past full size, then settle onto it.
        let u = appear.timer.fraction() - 1.0;
        let scale = 1.0 + (APPEAR_OVERSHOOT + 1.0) * u * u * u + APPEAR_OVERSHOOT * u * u;
        transform.scale = Vec3::splat(scale);
    }
}
