//! The play area: the ground's collider, the walls around it, its bounds, and sunlight.
//! The visible ground is drawn by `ground.rs`.

use bevy::light::CascadeShadowConfigBuilder;
use bevy::prelude::*;

use crate::physics::{box_outline, ground, outline_walls};

/// Width and depth of the square ground plane.
pub const GROUND_SIZE: f32 = 2.5;
/// Thickness of the invisible walls around the ground.
const WALL_THICKNESS: f32 = 0.1;
/// Height of the invisible walls around the ground.
const WALL_HEIGHT: f32 = 1.0;
/// Length cut off from corners to form a 45-degree wall (stops things from getting stuck).
const WALL_BEVEL: f32 = 0.15;
/// Brightness of the sunlight.
const SUN_ILLUMINANCE: f32 = 12_000.0;
/// Farthest distance from camera that gets shadows. Bevy's default (150) is way bigger than needed here.
const SHADOW_MAX_DISTANCE: f32 = 8.0;
/// Shadow cascades: fewer means fewer shadow passes per frame.
const SHADOW_CASCADES: usize = 1;
/// Sun pitch and yaw, in radians.
const SUN_ANGLES: (f32, f32) = (-std::f32::consts::FRAC_PI_4, -std::f32::consts::FRAC_PI_4);

/// Plugin that spawns the ground, its bounding walls and the sun, and provides `ArenaBounds`.
pub struct ArenaPlugin;

impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        let outline = box_outline(Vec2::ZERO, Vec2::splat(GROUND_SIZE * 0.5), WALL_BEVEL);
        app.insert_resource(ArenaBounds::new(outline))
            .add_systems(Startup, spawn_arena);
    }
}

/// The floor inside the arena walls, seen from above (x = world X, y = world Z).
#[derive(Resource, Debug, Clone)]
pub struct ArenaBounds {
    /// Inner wall corners in order around the arena; must be convex.
    outline: Vec<Vec2>,
}

impl ArenaBounds {
    /// Bounds enclosed by the convex `outline` of inner wall corners.
    pub fn new(outline: Vec<Vec2>) -> Self {
        Self { outline }
    }

    /// Inner wall corners in order around the arena.
    pub fn outline(&self) -> &[Vec2] {
        &self.outline
    }

    /// Smallest and largest corners of the axis-aligned box around the arena.
    pub fn min_max(&self) -> (Vec2, Vec2) {
        self.outline.iter().fold(
            (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
            |(min, max), &p| (min.min(p), max.max(p)),
        )
    }

    /// Distance from `point` to the nearest wall, or `None` if the point is outside the walls.
    pub fn wall_distance(&self, point: Vec2) -> Option<f32> {
        let edges = (0..self.outline.len())
            .map(|i| (self.outline[i], self.outline[(i + 1) % self.outline.len()]));
        let (mut left, mut right, mut nearest) = (false, false, f32::INFINITY);
        for (a, b) in edges {
            let along = b - a;
            let side = along.perp_dot(point - a);
            left |= side > 0.0;
            right |= side < 0.0;
            let length_sq = along.length_squared();
            let t = if length_sq > f32::EPSILON {
                ((point - a).dot(along) / length_sq).clamp(0.0, 1.0)
            } else {
                0.0
            };
            nearest = nearest.min(point.distance(a + along * t));
        }
        // Inside a convex outline, the point is on the same side of every edge.
        (!(left && right)).then_some(nearest)
    }
}

/// Spawns the ground collider, walls along the arena bounds, and a directional light.
fn spawn_arena(mut commands: Commands, bounds: Res<ArenaBounds>) {
    // Ground collider.
    commands.spawn(ground(0.0));

    // Invisible walls around the ground plane.
    for wall in outline_walls(bounds.outline(), WALL_THICKNESS, WALL_HEIGHT) {
        commands.spawn(wall);
    }

    // Sunlight.
    commands.spawn((
        DirectionalLight {
            illuminance: SUN_ILLUMINANCE,
            shadow_maps_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: SHADOW_CASCADES,
            maximum_distance: SHADOW_MAX_DISTANCE,
            ..default()
        }
        .build(),
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, SUN_ANGLES.0, SUN_ANGLES.1, 0.0)),
    ));
}
