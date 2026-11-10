//! Performance readout: an FPS and frame-time overlay, plus a key to toggle vsync.
//!
//! F3 shows or hides the overlay. F4 switches between vsync on (`AutoVsync`) and off
//! (`AutoNoVsync`), which shows how much headroom there is and how much input lag vsync adds.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow};

/// Whether the overlay is visible when the game starts.
const SHOW_ON_START: bool = false;
/// Key that shows or hides the overlay.
const TOGGLE_OVERLAY_KEY: KeyCode = KeyCode::F3;
/// Key that switches vsync on and off.
const TOGGLE_VSYNC_KEY: KeyCode = KeyCode::F4;
/// Seconds between overlay refreshes, so the numbers are readable.
const REFRESH_SECONDS: f32 = 0.25;
/// Frames kept for the average and worst frame time (about two seconds at 60 FPS).
const HISTORY_FRAMES: usize = 120;
/// Overlay text size in logical pixels.
const FONT_SIZE: f32 = 14.0;
/// Overlay distance from the top-left corner of the window, in logical pixels.
const MARGIN: f32 = 8.0;
/// Inner padding of the overlay box, in logical pixels.
const PADDING: f32 = 6.0;
/// Overlay text colour.
const TEXT_COLOR: Color = Color::srgb(0.95, 0.95, 0.95);
/// Overlay background colour.
const BACKGROUND_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
/// Draw order that keeps the overlay above every other UI layer.
const OVERLAY_Z: i32 = i32::MAX - 1;

/// Plugin that measures frame times and shows them in a corner overlay.
pub struct DiagnosticsOverlayPlugin;

impl Plugin for DiagnosticsOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::new(HISTORY_FRAMES))
            .add_systems(Startup, spawn_overlay)
            .add_systems(Update, (handle_keys, update_overlay));
    }
}

/// Marker for the overlay's text.
#[derive(Component)]
struct FrameTimeText;

/// Spawns the overlay box in the top-left corner.
fn spawn_overlay(mut commands: Commands) {
    commands.spawn((
        FrameTimeText,
        Text::new(""),
        TextFont { font_size: FONT_SIZE.into(), ..default() },
        TextColor(TEXT_COLOR),
        BackgroundColor(BACKGROUND_COLOR),
        Node {
            position_type: PositionType::Absolute,
            top: px(MARGIN),
            left: px(MARGIN),
            padding: UiRect::all(px(PADDING)),
            ..default()
        },
        GlobalZIndex(OVERLAY_Z),
        if SHOW_ON_START { Visibility::Inherited } else { Visibility::Hidden },
    ));
}

/// Toggles the overlay and vsync from the keyboard.
fn handle_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut overlay: Query<&mut Visibility, With<FrameTimeText>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) {
    if keys.just_pressed(TOGGLE_OVERLAY_KEY) {
        for mut visibility in &mut overlay {
            visibility.toggle_inherited_hidden();
        }
    }
    if keys.just_pressed(TOGGLE_VSYNC_KEY) {
        let Ok(mut window) = window.single_mut() else { return };
        window.present_mode = match window.present_mode {
            PresentMode::AutoNoVsync => PresentMode::AutoVsync,
            _ => PresentMode::AutoNoVsync,
        };
        info!("Present mode is now {:?}", window.present_mode);
    }
}

/// Writes the FPS, average and worst frame time, and vsync state into the overlay.
///
/// The worst frame matters most: a steady 60 FPS with one 50 ms frame every second feels laggy.
fn update_overlay(
    time: Res<Time<Real>>,
    mut since_refresh: Local<f32>,
    store: Res<DiagnosticsStore>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut texts: Query<(&mut Text, &InheritedVisibility), With<FrameTimeText>>,
) {
    *since_refresh += time.delta_secs();
    if *since_refresh < REFRESH_SECONDS {
        return;
    }
    *since_refresh = 0.0;

    let Some(frame_time) = store.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME) else { return };
    let average_ms = frame_time.average().unwrap_or(0.0);
    let worst_ms = frame_time.values().copied().fold(0.0, f64::max);
    let fps = if average_ms > 0.0 { 1000.0 / average_ms } else { 0.0 };
    let vsync = match window.single().map(|w| w.present_mode) {
        Ok(PresentMode::AutoNoVsync | PresentMode::Immediate | PresentMode::Mailbox) => "off",
        Ok(_) => "on",
        Err(_) => "?",
    };

    for (mut text, visible) in &mut texts {
        if visible.get() {
            text.0 = format!(
                "{fps:.0} FPS  avg {average_ms:.1} ms  worst {worst_ms:.1} ms  vsync {vsync} (F4)"
            );
        }
    }
}