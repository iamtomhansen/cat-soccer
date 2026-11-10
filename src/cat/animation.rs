//! Drives the cat's idle/walk blend and random idle fidgets from its movement.

use std::time::Duration;

use bevy::gltf::Gltf;
use bevy::prelude::*;

use super::movement::{move_cat, CatMotion, MoveState, CAT_SPEED};
use super::Cat;
use crate::animation::{AnimationConfig, ModelAnimations};
use crate::rng::Rng;

/// glTF name of the idle loop.
const IDLE_CLIP: &str = "Idle";
/// glTF name of the walk loop.
pub const WALK_CLIP: &str = "CatWalk";
/// Fidget clips layered over the idle, each with the bones (and descendants) it may move.
const FIDGETS: &[(&str, &[&str])] = &[
    ("eartwitch.l", &["ear.l"]),
    ("ear_twitch.r", &["ear.r"]),
    ("tailswish", &["tail1"]),
    ("tailtwitch", &["tail3"]),
];
/// Seconds to fully blend between idle and walk.
const BLEND_SECONDS: f32 = 0.25;
/// Walk clip playback rate at `CAT_SPEED`; slower movement scales it down proportionally.
const WALK_ANIM_SPEED_AT_TOP: f32 = 2.0;
/// Slowest playback rate of the walk clip, so legs keep moving while blending out.
const MIN_WALK_ANIM_SPEED: f32 = 0.1;
/// Shortest wait between fidgets, in seconds.
const FIDGET_INTERVAL_MIN: f32 = 3.0;
/// Longest wait between fidgets, in seconds.
const FIDGET_INTERVAL_MAX: f32 = 8.0;
/// Seconds a fidget takes to fade in and out.
const FIDGET_FADE_SECONDS: f32 = 0.15;
/// Idle weight needed before a fidget may start.
const FIDGET_MIN_IDLE_WEIGHT: f32 = 0.9;
/// Cap on a layer's share of its bones, keeping the weight conversion finite.
const MAX_LAYER_SHARE: f32 = 0.99;

/// The plugin that sets up and runs the cat's animations.
pub struct CatAnimationPlugin;

impl Plugin for CatAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (start_base_loops, update_cat_animations.after(move_cat)));
    }
}

/// Builds the cat's clip list for its model root.
pub fn cat_animation_config(gltf: Handle<Gltf>) -> AnimationConfig {
    FIDGETS.iter().fold(
        AnimationConfig::new(gltf).clip(IDLE_CLIP).clip(WALK_CLIP),
        |config, (name, bones)| config.masked_clip(name, bones),
    )
}

/// Per-cat animation blend and fidget state.
#[derive(Component, Debug)]
pub struct CatAnimator {
    /// Linear 0 (idle) to 1 (walk) blend progress.
    pub walk_blend: f32,
    /// Counts down to the next fidget.
    pub fidget_timer: Timer,
}

impl Default for CatAnimator {
    fn default() -> Self {
        let first_wait = Duration::from_secs_f32(FIDGET_INTERVAL_MAX);
        Self { walk_blend: 0.0, fidget_timer: Timer::new(first_wait, TimerMode::Once) }
    }
}

/// Starts the idle and walk loops whenever a cat's animations are (re)linked.
fn start_base_loops(
    cats: Query<&ModelAnimations, (With<Cat>, Changed<ModelAnimations>)>,
    mut players: Query<&mut AnimationPlayer>,
) {
    for anims in &cats {
        let Ok(mut player) = players.get_mut(anims.player) else { continue };
        if let Some(idle) = anims.clip(IDLE_CLIP) {
            player.play(idle.node).repeat().set_weight(1.0);
        }
        if let Some(walk) = anims.clip(WALK_CLIP) {
            player.play(walk.node).repeat().set_weight(0.0);
        }
    }
}

/// Blends idle and walk from movement, scales walk speed, and layers fidgets on top.
fn update_cat_animations(
    time: Res<Time>,
    mut rng: ResMut<Rng>,
    mut cats: Query<(&CatMotion, &ModelAnimations, &mut CatAnimator), With<Cat>>,
    mut players: Query<&mut AnimationPlayer>,
) {
    for (motion, anims, mut animator) in &mut cats {
        let Ok(mut player) = players.get_mut(anims.player) else { continue };
        let (Some(idle), Some(walk)) = (anims.clip(IDLE_CLIP), anims.clip(WALK_CLIP)) else {
            continue;
        };

        // Move the blend towards the current movement state.
        let target = if motion.state == MoveState::Walking { 1.0 } else { 0.0 };
        let step = time.delta_secs() / BLEND_SECONDS;
        animator.walk_blend += (target - animator.walk_blend).clamp(-step, step);
        let walk_weight = smoothstep(animator.walk_blend);
        let idle_weight = 1.0 - walk_weight;

        // Base weights sum to 1, so a layer's weight maps directly to its share of its bones.
        if let Some(active) = player.animation_mut(idle.node) {
            active.set_weight(idle_weight);
        }
        if let Some(active) = player.animation_mut(walk.node) {
            let anim_speed =
                (motion.speed() / CAT_SPEED * WALK_ANIM_SPEED_AT_TOP).max(MIN_WALK_ANIM_SPEED);
            active.set_weight(walk_weight).set_speed(anim_speed);
        }

        // Fade playing fidgets in and out, scaled down as the cat starts walking.
        let mut fidgeting = false;
        for (name, _) in FIDGETS {
            let Some(clip) = anims.clip(name) else { continue };
            let Some(active) = player.animation_mut(clip.node) else { continue };
            if active.is_finished() {
                player.stop(clip.node);
                continue;
            }
            fidgeting = true;
            let t = active.seek_time();
            let fade = (t.min(clip.duration - t) / FIDGET_FADE_SECONDS).clamp(0.0, 1.0);
            active.set_weight(layer_weight(fade * idle_weight));
        }

        // Start a random fidget when the timer runs out and the cat is resting.
        animator.fidget_timer.tick(time.delta());
        if animator.fidget_timer.just_finished() {
            animator.fidget_timer = random_fidget_timer(&mut rng);
            if !fidgeting && idle_weight >= FIDGET_MIN_IDLE_WEIGHT {
                let (name, _) = FIDGETS[rng.index(FIDGETS.len())];
                if let Some(clip) = anims.clip(name) {
                    player.start(clip.node).set_weight(0.0);
                }
            }
        }
    }
}

/// Converts a layer's desired share of its bones into a weight against base weights summing to 1.
fn layer_weight(share: f32) -> f32 {
    let share = share.clamp(0.0, MAX_LAYER_SHARE);
    share / (1.0 - share)
}

/// Eases a 0 to 1 value in and out.
fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A one-shot timer for a random wait until the next fidget.
fn random_fidget_timer(rng: &mut Rng) -> Timer {
    let secs = rng.range_f32(FIDGET_INTERVAL_MIN, FIDGET_INTERVAL_MAX);
    Timer::new(Duration::from_secs_f32(secs), TimerMode::Once)
}
