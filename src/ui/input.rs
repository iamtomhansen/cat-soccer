//! On-screen touch controls for player input.

use bevy::prelude::*;
use touchy::prelude::*;
use touchy::theme::{Shade, Skin, palette};

// const STICK_BG: Color = Color::srgba(0.16, 0.10, 0.12, 0.25);
// const STICK_BORDER: Color = Color::srgba(0.30, 0.34, 0.42, 0.5);
// const STICK_THUMB: Color = Color::srgba(0.26, 0.28, 0.33, 0.6);
// const STICK_HIGHLIGHT: Color = Color::srgba(0.4, 0.20, 0.60, 0.6); 

/// The plugin that sets up and runs the user input.
pub struct UiInputPlugin;

impl Plugin for UiInputPlugin {
    fn build(&self, app: &mut App) {
        // Style the joystick to be translucent.
        let stick = JoystickTokens::default();
        app.world_mut()
            .resource_mut::<Theme>()
            .set_derived(stick.bg, palette::SURFACE, Shade::Alpha(0.20))
            .set_derived(stick.border, palette::EDGE, Shade::Alpha(0.5))
            .set_derived(stick.thumb, palette::CONTROL, Shade::Alpha(0.6));
        // Add the control system on startup.
        app.add_systems(Startup, (
            populate_thumb,
            setup_touch_controls
        ));
    }
}

/// Startup system to spawn a floating joystick to control the cat.
fn setup_touch_controls(mut commands: Commands, layers: Res<UiLayers>) {
    commands.entity(layers.screen).with_children(|screen| {
        let config = JoystickConfig::new()
            .floating(true)
            .radius(72.0);
        screen.joystick(config);
    });
}

/// Adds the custom joystick thumb image to the theme.
fn populate_thumb(server: Res<AssetServer>, mut theme: ResMut<Theme>) {
    let thumb = JoystickTokens::default().thumb;
    theme.set_skin(thumb, Skin::new(server.load("ui/catstick.png")));
}