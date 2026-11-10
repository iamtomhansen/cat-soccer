//! Cat sounds: a soft paw tap each time a foot lands in the walk cycle, and a shuffling scuff when
//! the cat starts or stops walking.

use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;

use super::animation::{CatAnimator, WALK_CLIP};
use super::movement::{CatMotion, MoveState, CAT_SPEED};
use super::Cat;
use crate::animation::ModelAnimations;
use crate::audio::{PlaySound, Synth};
use crate::rng::Rng;

/// Times in the walk clip, in seconds, when a paw lands, with whether it's a hind paw. Read from
/// when each ankle IK target in cat.glb comes down: front left, back left, front right, back right.
const FOOTFALLS: [(f32, bool); 4] = [(0.25, false), (0.50, true), (0.71, false), (0.97, true)];
/// Quietest step that is still played, as walk blend times speed fraction; fainter ones are skipped.
const MIN_STEP_LOUDNESS: f32 = 0.05;
/// The base paw tap, at full walk speed: a short tick of noise over a soft low pat.
const TAP: Synth = Synth {
    pitch: 220.0,
    end_pitch: 220.0,
    tone: 0.8,
    noise: 0.2,
    brightness: 0.3,
    decay: 0.02,
    delay: 0.0,
    volume: 0.3,
    seed: 0,
};
/// Noise brightness of a hind paw tap; hind paws land a little heavier and duller than `TAP`.
const HIND_BRIGHTNESS: f32 = 0.35;
/// Random variation of each tap's brightness, as a fraction either way.
const TAP_JITTER: f32 = 0.15;
/// The scuff when the cat starts or stops: a longer, duller brush of noise.
const SCUFF: Synth = Synth {
    pitch: 220.0,
    end_pitch: 220.0,
    tone: 0.5,
    noise: 0.5,
    brightness: 0.15,
    decay: 0.06,
    delay: 0.0,
    volume: 0.3,
    seed: 0,
};

/// Plugin for the cat's footstep sounds.
pub struct CatSoundPlugin;

impl Plugin for CatSoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (footstep_sounds, scuff_sounds));
    }
}

/// Plays a tap for each footfall the walk clip passed since last frame.
/// Loudness animation blend amount so steps fade in and out with the walk.
fn footstep_sounds(
    cats: Query<(&CatMotion, &CatAnimator, &ModelAnimations), With<Cat>>,
    players: Query<&AnimationPlayer>,
    mut rng: ResMut<Rng>,
    mut sounds: MessageWriter<PlaySound>,
) {
    for (motion, animator, anims) in &cats {
        let Some(walk) = anims.clip(WALK_CLIP) else { continue; };
        let Ok(player) = players.get(anims.player) else { continue; };
        let Some(active) = player.animation(walk.node) else { continue; };
        let Some(from) = active.last_seek_time() else { continue; };
        let to = active.seek_time();

        let loudness = animator.walk_blend * (motion.speed() / CAT_SPEED).min(1.0);
        if loudness < MIN_STEP_LOUDNESS {
            continue;
        }
        for &(time, hind) in &FOOTFALLS {
            // The clip loops, so the passed span may wrap from the end back to the start.
            let passed = if from <= to { from < time && time <= to } else { from < time || time <= to };
            if passed {
                sounds.write(PlaySound(paw_tap(hind, loudness, &mut rng)));
            }
        }
    }
}

/// Plays a scuff when a cat starts or stops walking.
fn scuff_sounds(
    cats: Query<(Entity, &CatMotion), With<Cat>>,
    mut last_states: Local<EntityHashMap<MoveState>>,
    mut rng: ResMut<Rng>,
    mut sounds: MessageWriter<PlaySound>,
) {
    for (cat, motion) in &cats {
        let before = last_states.insert(cat, motion.state);
        if before.is_some_and(|state| state != motion.state) {
            sounds.write(PlaySound(scuff(&mut rng)));
        }
    }
}

/// Builds one paw tap from `TAP`, scaled by `loudness` from 0 (silent) to 1 (full speed).
pub fn paw_tap(hind: bool, loudness: f32, rng: &mut Rng) -> Synth {
    let brightness = if hind { HIND_BRIGHTNESS } else { TAP.brightness };
    Synth {
        brightness: brightness * (1.0 + rng.range_f32(-TAP_JITTER, TAP_JITTER)),
        volume: TAP.volume * loudness.min(1.0),
        seed: rng.u64(),
        ..TAP
    }
}

/// Builds a scuff from `SCUFF`.
pub fn scuff(rng: &mut Rng) -> Synth {
    Synth { seed: rng.u64(), ..SCUFF }
}
