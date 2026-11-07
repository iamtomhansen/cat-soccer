//! The game overlay UI, giving feedback to the user.

use bevy::prelude::*;
use touchy::prelude::*;

const WELCOME_WIDTH: f32 = 400.0;
const WELCOME_TITLE: &str = "Cat Sandbox";
const WELCOME_BODY: &[&str] = &[
    "Welcome to the physics cat demo.",
    "Use the virtual joystick to explore.",
];

/// The welcome card that is shown at the start of the game.
#[derive(Component)]
struct WelcomeCard;

/// The plugin that sets up and runs the HUD.
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hud);
    }
}

/// Startup system to spawn the HUD (just a welcome card for now).
fn setup_hud(mut commands: Commands) {
    commands
        .spawn((WelcomeCard, UiLayer::Overlay, Column, Card))
        .node(|node| node.width = px(WELCOME_WIDTH))
        .with_children(|card| {
            card.heading(WELCOME_TITLE);
            for line in WELCOME_BODY {
                card.text(*line);
            }
            card.row(|row| {
                row.spacer();
                row.button(ButtonConfig::new("Explore"))
                    .observe(close_welcome);
            });
        });
}

/// Toggle the welcome card.
fn close_welcome(
    _: On<Activate>,
    query: Query<Entity, With<WelcomeCard>>,
    mut commands: Commands,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}