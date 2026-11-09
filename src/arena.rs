//! The play area: ground plane and its collider, the walls around it, and sunlight.

use bevy::prelude::*;

use crate::physics::{box_walls, ground};

/// Width and depth of the square ground plane.
const GROUND_SIZE: f32 = 2.0;
/// Colour of the ground.
const GROUND_COLOR: Color = Color::srgb(0.2, 0.25, 0.2);
/// Thickness of the invisible walls around the ground.
const WALL_THICKNESS: f32 = 0.1;
/// Height of the invisible walls around the ground.
const WALL_HEIGHT: f32 = 1.0;
/// Length cut off from corners to form a 45-degree wall (stops things from getting stuck).
const WALL_BEVEL: f32 = 0.15;
/// Brightness of the sunlight.
const SUN_ILLUMINANCE: f32 = 12_000.0;
/// Sun pitch and yaw, in radians.
const SUN_ANGLES: (f32, f32) = (-std::f32::consts::FRAC_PI_4, -std::f32::consts::FRAC_PI_4);

/// Plugin that spawns the ground, its bounding walls and the sun.
pub struct ArenaPlugin;

impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_arena);
    }
}

/// Spawns the ground plane, walls along its edges, and a directional light.
fn spawn_arena(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ground plane.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(GROUND_SIZE, GROUND_SIZE))),
        MeshMaterial3d(materials.add(GROUND_COLOR)),
        Transform::default(),
    ));
    commands.spawn(ground(0.0));

    // Invisible walls around the ground plane.
    for wall in box_walls(Vec2::ZERO, Vec2::splat(GROUND_SIZE * 0.5), WALL_BEVEL, WALL_THICKNESS, WALL_HEIGHT) {
        commands.spawn(wall);
    }

    // Sunlight.
    commands.spawn((
        DirectionalLight {
            illuminance: SUN_ILLUMINANCE,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, SUN_ANGLES.0, SUN_ANGLES.1, 0.0)),
    ));
}
