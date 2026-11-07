//! A plugin to manage the cat's animations.

use bevy::prelude::*;
use touchy::prelude::*;

/// The plugin that sets up and runs the user input.
pub struct UiInputPlugin;

impl Plugin for UiInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_touch_controls);
    }
}

fn setup_touch_controls(mut commands: Commands, layers: Res<UiLayers>) {
    commands.entity(layers.screen).with_children(|screen| {
        let config = JoystickConfig::new()
            .floating(true)
            .radius(72.0);
        screen.joystick(config);
    });
}