//! Entry point for the UI module.

pub mod hud;
pub mod input;

use bevy::prelude::*;
use hud::HudPlugin;
use input::UiInputPlugin;

/// The plugin that sets up all game UI.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((UiInputPlugin, HudPlugin));
    }
}