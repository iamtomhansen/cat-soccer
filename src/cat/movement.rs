//! The movement logic for the cat.

use bevy::prelude::*;
use touchy::prelude::*;
use super::Cat;

const CAT_SPEED: f32 = 3.0;

/// The plugin that sets up and runs the cat movement.
pub struct CatMovementPlugin;

impl Plugin for CatMovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, move_cat);
    }
}

/// Move the cat based on currenty input.
fn move_cat(
    time: Res<Time>,
    sticks: Query<&JoystickValue>,
    mut cats: Query<&mut Transform, With<Cat>>,
) {
    let Ok(stick) = sticks.single() else {
        return;
    };

    if stick.0.length_squared() < 0.001 {
        return;
    }

    // Convert 2D screen joystick (X = right/left, Y = up/down)
    // to 3D world space (X = X, Y = 0, Z = -Y)
    let move_dir = Vec3::new(stick.0.x, 0.0, -stick.0.y).normalize();

    for mut transform in &mut cats {
        // Translation
        transform.translation += move_dir * CAT_SPEED * time.delta_secs();

        // Smooth rotation towards movement direction
        let target_rot = Transform::default().looking_to(move_dir, Vec3::Y).rotation;
        transform.rotation = transform.rotation.slerp(target_rot, 12.0 * time.delta_secs());
    }
}