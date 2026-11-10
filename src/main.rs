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

use bevy::asset::AssetMetaCheck;
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

/// Window title, shown on desktop.
const WINDOW_TITLE: &str = "Cat soccer";
/// CSS selector of the `<canvas>` in `wasm/index.html` that the game draws into on the web.
const WEB_CANVAS: &str = "#bevy";

fn main() {
    App::new()
        // Dependencies.
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: WINDOW_TITLE.into(),
                        // Web only: draw into the page's canvas and fill its parent.
                        canvas: Some(WEB_CANVAS.into()),
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                })
                // Don't look for `.meta` files: on a static web host each lookup is a 404.
                .set(AssetPlugin { meta_check: AssetMetaCheck::Never, ..default() }),
        )
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
