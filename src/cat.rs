//! Entry point for the cat-related code.

pub mod animation;
pub mod movement;

use bevy::prelude::*;

use animation::{cat_animation_config, CatAnimationPlugin, CatAnimator};
use movement::{CatMotion, CatMovementPlugin};
use avian3d::prelude::{CustomPositionIntegration, RigidBody, SleepingDisabled};

use crate::model::{ModelConfig, ModelSize};
use crate::physics::FitCollider;

/// Path to the cat's glTF model.
const CAT_MODEL_PATH: &str = "models/cat.glb";
/// Height of the cat in world units.
pub const CAT_HEIGHT: f32 = 0.25;
/// Turn applied to the model so it faces the cat's forward direction.
const CAT_MODEL_YAW_DEGREES: f32 = 180.0;

/// Marker for the cat entity.
#[derive(Component)]
#[require(CatMotion, CatAnimator)]
pub struct Cat;

/// The plugin that sets up and runs the cat.
pub struct CatPlugin;

impl Plugin for CatPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            CatMovementPlugin,
            CatAnimationPlugin,
        ))
            .add_systems(Startup, spawn_cat);
    }
}

/// Startup system to spawn the cat entity.
fn spawn_cat(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Cat,
        Transform::default(),
        // Kinematic: Moved by the CatMotion system, not physics.
        RigidBody::Kinematic,
        // Avian doesn't see "velocity" on manual entities, so force cat physics to stay "awake".
        SleepingDisabled,
        CustomPositionIntegration,
        FitCollider::Box,
        ModelConfig::new(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(CAT_MODEL_PATH)),
            ModelSize::Height(CAT_HEIGHT),
        )
            .rotation_degrees(CAT_MODEL_YAW_DEGREES),
        cat_animation_config(asset_server.load(CAT_MODEL_PATH)),
    ));
}
