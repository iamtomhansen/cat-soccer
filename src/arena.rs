//! The play area: ground plane, the walls around it, and sunlight.

use bevy::prelude::*;

use crate::physics::box_walls;

/// Width and depth of the square ground plane.
const GROUND_SIZE: f32 = 2.0;
/// Colour of the ground.
const GROUND_COLOR: Color = Color::srgb(0.2, 0.25, 0.2);
/// Thickness of the invisible walls around the ground.
const WALL_THICKNESS: f32 = 0.5;
/// Height of the invisible walls around the ground.
const WALL_HEIGHT: f32 = 1.0;
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
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(GROUND_SIZE, GROUND_SIZE))),
        MeshMaterial3d(materials.add(GROUND_COLOR)),
        Transform::default(),
    ));

    for wall in box_walls(Vec3::ZERO, Vec2::splat(GROUND_SIZE * 0.5), WALL_THICKNESS, WALL_HEIGHT) {
        commands.spawn(wall);
    }

    commands.spawn((
        DirectionalLight {
            illuminance: SUN_ILLUMINANCE,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, SUN_ANGLES.0, SUN_ANGLES.1, 0.0)),
    ));
}
