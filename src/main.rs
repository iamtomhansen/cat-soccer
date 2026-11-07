mod camera;
mod cat;
mod model;
mod ui;

use bevy::prelude::*;
use touchy::prelude::*;

use camera::CameraPlugin;
use cat::{Cat, CatPlugin};
use model::ModelNormalizePlugin;
use ui::UiPlugin;

fn main() {
    App::new()
        // Dependencies.
        .add_plugins(DefaultPlugins)
        .add_plugins(TouchyPlugins)
        
        // Crate plugins.
        .add_plugins((
            CameraPlugin,
            CatPlugin,
            ModelNormalizePlugin,
            UiPlugin,
        ))
        
        // Game setup.
        .add_systems(Startup, setup_world_environment)
        .run();
}

/// Quick and dirty "spawn everything" system to get a basic world set up.
fn setup_world_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ground plane so we can perceive motion and depth
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(2.0, 2.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.25, 0.2))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // Directional sunlight
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_4,
            -std::f32::consts::FRAC_PI_4,
            0.0,
        )),
    ));
}
