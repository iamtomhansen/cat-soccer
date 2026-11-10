//! Goal sounds: a rising chime when a goal is scored, pitched higher for each goal in a quick
//! streak, and a soft poof as the goal vanishes and moves.

use bevy::prelude::*;

use super::{GoalMoved, GoalScored};
use crate::audio::{PlaySound, Synth};
use crate::rng::Rng;

/// One note of the chime, at the root pitch (A5): a plain ringing sine.
const CHIME_NOTE: Synth = Synth {
    pitch: 880.0,
    end_pitch: 880.0,
    tone: 1.0,
    noise: 0.0,
    brightness: 0.0,
    decay: 0.3,
    delay: 0.0,
    volume: 0.1,
    seed: 0,
};
/// Notes of the chime in semitones above the root: a rising major arpeggio up to the octave.
const CHIME_NOTES: [f32; 4] = [0.0, 4.0, 7.0, 12.0];
/// Time between the chime's notes, in seconds.
const CHIME_NOTE_SPACING_SECONDS: f32 = 0.07;
/// Longest gap between goals, in seconds, that still continues a streak.
const STREAK_SECONDS: f32 = 8.0;
/// How much higher each goal in a streak pitches the chime, in semitones.
const STREAK_STEP_SEMITONES: f32 = 2.0;
/// Most streak steps the chime climbs before it stops getting higher.
const MAX_STREAK_STEPS: u32 = 6;
/// The poof as the goal vanishes: a soft puff of dull noise.
const POOF: Synth = Synth {
    pitch: 0.0,
    end_pitch: 0.0,
    tone: 0.0,
    noise: 1.0,
    brightness: 0.25,
    decay: 0.1,
    delay: 0.0,
    volume: 0.3,
    seed: 0,
};

/// Plugin for the goal's sounds.
pub struct GoalSoundPlugin;

impl Plugin for GoalSoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GoalStreak>()
            .add_systems(Update, (goal_chimes, poof_sounds));
    }
}

/// The current run of goals scored in quick succession.
#[derive(Resource, Default)]
struct GoalStreak {
    /// Goals in the streak before the latest one.
    steps: u32,
    /// When the latest goal was scored, in virtual seconds, if any has been.
    last: Option<f32>,
}

/// Plays a chime for each goal, higher for each goal in a streak.
fn goal_chimes(
    mut scored: MessageReader<GoalScored>,
    mut streak: ResMut<GoalStreak>,
    time: Res<Time<Virtual>>,
    mut sounds: MessageWriter<PlaySound>,
) {
    let now = time.elapsed_secs();
    for _ in scored.read() {
        let continues = streak.last.is_some_and(|t| now - t < STREAK_SECONDS);
        streak.steps = if continues { (streak.steps + 1).min(MAX_STREAK_STEPS) } else { 0 };
        streak.last = Some(now);
        let transpose = streak.steps as f32 * STREAK_STEP_SEMITONES;
        sounds.write_batch(chime(transpose).map(PlaySound));
    }
}

/// Plays a poof each time a goal vanishes to move.
fn poof_sounds(
    mut moved: MessageReader<GoalMoved>,
    mut rng: ResMut<Rng>,
    mut sounds: MessageWriter<PlaySound>,
) {
    for _ in moved.read() {
        sounds.write(PlaySound(poof(&mut rng)));
    }
}

/// Converts a pitch offset in semitones to a frequency multiple.
fn semitones(offset: f32) -> f32 {
    2.0_f32.powf(offset / 12.0)
}

/// Builds the notes of the goal chime from `CHIME_NOTE`, transposed up by `transpose` semitones,
/// each delayed a little more than the last.
pub fn chime(transpose: f32) -> impl Iterator<Item = Synth> {
    CHIME_NOTES.into_iter().enumerate().map(move |(i, note)| {
        let pitch = CHIME_NOTE.pitch * semitones(note + transpose);
        Synth {
            pitch,
            end_pitch: pitch,
            delay: i as f32 * CHIME_NOTE_SPACING_SECONDS,
            ..CHIME_NOTE
        }
    })
}

/// Builds the poof from `POOF`.
pub fn poof(rng: &mut Rng) -> Synth {
    Synth { seed: rng.u64(), ..POOF }
}
