//! A cat that walks around a ground plane, driven by an on-screen joystick.

#![allow(dead_code)]

mod animation;
mod arena;
mod audio;
mod ball;
mod camera;
mod cat;
mod diagnostics;
mod goal;
mod ground;
mod model;
mod physics;
mod rng;
mod skybox;
mod ui;

use bevy::prelude::*;
use touchy::prelude::*;

use animation::ModelAnimationPlugin;
use arena::ArenaPlugin;
use audio::SoundPlugin;
use ball::BallPlugin;
use camera::CameraPlugin;
use cat::CatPlugin;
use model::ModelNormalizePlugin;
use rng::RngPlugin;
use skybox::SkyboxPlugin;
use ui::UiPlugin;

fn main() {
    App::new()
        // Dependencies.
        .add_plugins(DefaultPlugins)
        .add_plugins(TouchyPlugins)
        
        // Crate plugins.
        .add_plugins((
            ArenaPlugin,
            BallPlugin,
            CameraPlugin,
            CatPlugin,
            diagnostics::DiagnosticsOverlayPlugin,
            goal::GoalPlugin,
            ground::GroundPlugin,
            ModelAnimationPlugin,
            ModelNormalizePlugin,
            physics::PhysicsPlugin,
            RngPlugin,
            SkyboxPlugin,
            SoundPlugin,
            UiPlugin,
        ))
        
        // Run.
        .run();
}
