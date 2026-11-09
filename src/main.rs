//! A cat that walks around a ground plane, driven by an on-screen joystick.

mod animation;
mod arena;
mod camera;
mod cat;
mod model;
mod physics;
mod rng;
mod ui;

use bevy::prelude::*;
use touchy::prelude::*;

use animation::ModelAnimationPlugin;
use arena::ArenaPlugin;
use camera::CameraPlugin;
use cat::CatPlugin;
use model::ModelNormalizePlugin;
use rng::RngPlugin;
use ui::UiPlugin;

fn main() {
    App::new()
        // Dependencies.
        .add_plugins(DefaultPlugins)
        .add_plugins(TouchyPlugins)
        
        // Crate plugins.
        .add_plugins((
            ArenaPlugin,
            CameraPlugin,
            CatPlugin,
            ModelAnimationPlugin,
            ModelNormalizePlugin,
            physics::PhysicsPlugin,
            RngPlugin,
            UiPlugin,
        ))
        
        // Run.
        .run();
}
