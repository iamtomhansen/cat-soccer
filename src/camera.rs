//! The game's 3d camera.

use bevy::prelude::*;
use crate::cat::Cat;

/// The plugin that sets up and runs the camera.
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(PostUpdate, camera_follow);
    }
}

/// The 3D camera that follow the cat.
#[derive(Component)]
pub struct FollowCamera {
    /// 3D offset from the cat.
    pub offset: Vec3,
    /// How quickly the camera moves to follow the cat.
    pub smooth_speed: f32,
}

impl Default for FollowCamera {
    fn default() -> Self {
        Self {
            offset: Vec3::new(0.0, 2.0, 4.5),
            smooth_speed: 6.0,
        }
    }
}

/// Startup system to spawn the camera.
fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        FollowCamera::default(),
    ));
}

// System to smoothly chase the cat with the camera.
fn camera_follow(
    time: Res<Time>,
    cats: Query<&Transform, (With<Cat>, Without<FollowCamera>)>,
    mut cameras: Query<(&mut Transform, &FollowCamera)>,
) {
    let Ok(cat_tf) = cats.single() else { return; };
    for (mut cam_tf, follow) in &mut cameras {
        let target_pos = cat_tf.translation + follow.offset;
        // Smooth exponential dampening
        cam_tf.translation = cam_tf.translation.lerp(
            target_pos,
            follow.smooth_speed * time.delta_secs(),
        );
        // Look slightly above the cat's feet
        cam_tf.look_at(cat_tf.translation + Vec3::Y * 0.5, Vec3::Y);
    }
}