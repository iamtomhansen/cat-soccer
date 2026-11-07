//! Entry point for the cat-related code.

// pub mod animation;
pub mod movement;

use bevy::prelude::*;

// use animation::CatAnimationPlugin;
use movement::CatMovementPlugin;
use crate::model::{ModelConfig, ModelSize};

/// Marker for the cat entity.
#[derive(Component)]
pub struct Cat;

/// The plugin that sets up and runs the cat.
pub struct CatPlugin;

impl Plugin for CatPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            CatMovementPlugin,
            // CatAnimationPlugin,
        ))
            .add_systems(Startup, spawn_cat);
    }
}

/// Startup system to spawn the cat entity.
fn spawn_cat(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Cat,
        Transform::from_xyz(0.0, 0.0, 0.0),
        ModelConfig::new(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/cat.glb")),
            ModelSize::Height(0.25),
        )
            .rotation_degrees(180.0)
    ));
}