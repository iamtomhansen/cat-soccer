//! Ball sounds: one procedural thud for kicks and bounces, shaped by how hard the hit was and what
//! the ball hit. Bounces are read from Avian's contacts, using each contact's impact speed.

use avian3d::prelude::*;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;

use super::Ball;
use crate::audio::{PlaySound, Synth};
use crate::cat::Cat;
use crate::goal::Goal;
use crate::physics::{GameLayer, Kicked};
use crate::rng::Rng;

/// Smallest kick, as a speed gain in m/s, that makes a sound; gentler dribbling touches are silent.
const KICK_MIN_SPEED_GAIN: f32 = 0.05;
/// Kick speed gain, in m/s, that makes the loudest thud.
const KICK_FULL_SPEED_GAIN: f32 = 1.5;
/// Shortest time between kick sounds of the same ball, since a pushed ball is kicked every frame.
const KICK_COOLDOWN_SECONDS: f32 = 0.15;
/// Smallest impact speed, in m/s, that makes a bounce sound. Above the ~0.15 m/s a resting ball
/// picks up from one physics step of gravity, so resting and rolling stay silent.
const BOUNCE_MIN_SPEED: f32 = 0.3;
/// Impact speed, in m/s, that makes the loudest bounce.
const BOUNCE_FULL_SPEED: f32 = 2.5;
/// Shortest time between bounce sounds of the same ball, so one impact can't sound twice.
const BOUNCE_COOLDOWN_SECONDS: f32 = 0.06;
/// Time after a kick during which the ball's bounces are silent, as the kick already thudded.
const KICK_MASKS_BOUNCE_SECONDS: f32 = 0.1;
/// How quickly a ball's recent-sound tally fades, as a time constant in seconds.
const RATTLE_DECAY_SECONDS: f32 = 0.6;
/// How much each recent sound quietens the next, so a ball rattling between a paw and a wall (a
/// kick and a bounce every few frames) settles to a soft patter rather than repeated loud thuds.
const RATTLE_DUCKING: f32 = 0.5;
/// The base thud, at full strength off a paw. Tune the sound here; hits scale it from this.
const THUD: Synth = Synth {
    pitch: 200.0,
    end_pitch: 110.0,
    tone: 0.8,
    noise: 0.2,
    brightness: 0.2,
    decay: 0.08,
    delay: 0.0,
    volume: 0.9,
    seed: 0,
};
/// Volume of the softest and the hardest hit, as a multiple of the base thud's.
const VOLUME_RANGE: (f32, f32) = (0.15, 1.0);
/// Decay of the softest and the hardest hit, as a multiple of the base thud's; hard hits ring longer.
const DECAY_RANGE: (f32, f32) = (0.5, 1.3);
/// Pitch of the softest and the hardest hit, as a multiple of the base thud's; hard hits are lower.
const PITCH_RANGE: (f32, f32) = (1.5, 1.0);
/// Random pitch variation of each thud, as a fraction either way.
const PITCH_JITTER: f32 = 0.05;
/// Pitch multiple and noise brightness of a kick: soft and muffled.
const PAW_COLOUR: (f32, f32) = (1.0, 0.2);
/// Pitch multiple and noise brightness of a bounce off the ground: dull and low.
const GROUND_COLOUR: (f32, f32) = (0.85, 0.3);
/// Pitch multiple and noise brightness of a bounce off a wall: brighter.
const WALL_COLOUR: (f32, f32) = (1.1, 0.6);
/// Pitch multiple and noise brightness of a bounce off a goal post: higher and brightest.
const POST_COLOUR: (f32, f32) = (1.3, 0.8);

/// Plugin for the ball's kick and bounce sounds.
pub struct BallSoundPlugin;

impl Plugin for BallSoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BallSoundTimes>()
            .add_systems(Update, kick_sounds)
            .add_systems(FixedPostUpdate, bounce_sounds.after(PhysicsSystems::Last));
    }
}

/// What the ball hit, which colours its thud.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    /// A paw: soft and muffled.
    Paw,
    /// The ground: dull and low.
    Ground,
    /// A wall or anything else hard: brighter.
    Wall,
    /// A goal post: higher and brightest.
    Post,
}

impl Surface {
    /// Pitch multiple and noise brightness this surface gives a thud.
    fn colour(self) -> (f32, f32) {
        match self {
            Surface::Paw => PAW_COLOUR,
            Surface::Ground => GROUND_COLOUR,
            Surface::Wall => WALL_COLOUR,
            Surface::Post => POST_COLOUR,
        }
    }
}

/// When each ball last made a kick or bounce sound, in virtual seconds, and how busy it has been.
#[derive(Resource, Default)]
struct BallSoundTimes {
    /// Last kick sound of each ball.
    kick: EntityHashMap<f32>,
    /// Last bounce sound of each ball.
    bounce: EntityHashMap<f32>,
    /// Each ball's fading tally of recent sounds, and when it was last updated.
    rattle: EntityHashMap<(f32, f32)>,
}

impl BallSoundTimes {
    /// Counts a new sound from `ball` at time `now` and returns the volume multiple to play it at, lower the
    /// more sounds the ball has made just before.
    fn duck(&mut self, ball: Entity, now: f32) -> f32 {
        let (tally, last) = self.rattle.get(&ball).copied().unwrap_or((0.0, now));
        let tally = tally * (-(now - last) / RATTLE_DECAY_SECONDS).exp();
        self.rattle.insert(ball, (tally + 1.0, now));
        1.0 / (1.0 + RATTLE_DUCKING * tally)
    }
}

/// Plays a thud for each kick, louder for harder kicks.
fn kick_sounds(
    mut kicks: MessageReader<Kicked>,
    balls: Query<(), With<Ball>>,
    time: Res<Time<Virtual>>,
    mut times: ResMut<BallSoundTimes>,
    mut rng: ResMut<Rng>,
    mut sounds: MessageWriter<PlaySound>,
) {
    let now = time.elapsed_secs();
    for kick in kicks.read() {
        if !balls.contains(kick.entity) || kick.speed_gain < KICK_MIN_SPEED_GAIN {
            continue;
        }
        if times.kick.get(&kick.entity).is_some_and(|&t| now - t < KICK_COOLDOWN_SECONDS) {
            continue;
        }
        times.kick.insert(kick.entity, now);
        let strength = (kick.speed_gain / KICK_FULL_SPEED_GAIN).clamp(0.0, 1.0);
        let mut synth = thud(strength, Surface::Paw, &mut rng);
        synth.volume *= times.duck(kick.entity, now);
        sounds.write(PlaySound(synth));
    }
}

/// Plays a thud for each hard enough impact between a ball and anything but the cat.
///
/// Runs after each physics step and reads the impact speed Avian measured before solving each
/// contact, so every bounce reports its own speed. Only contacts the solver actually pushed on
/// count, which skips speculative contacts that haven't touched yet.
fn bounce_sounds(
    collisions: Collisions,
    balls: Query<Entity, With<Ball>>,
    cats: Query<(), With<Cat>>,
    goals: Query<(), With<Goal>>,
    layers: Query<&CollisionLayers>,
    time: Res<Time<Virtual>>,
    mut times: ResMut<BallSoundTimes>,
    mut rng: ResMut<Rng>,
    mut sounds: MessageWriter<PlaySound>,
) {
    let now = time.elapsed_secs();
    for ball in &balls {
        let recent = |map: &EntityHashMap<f32>, window: f32| {
            map.get(&ball).is_some_and(|&t| now - t < window)
        };
        if recent(&times.bounce, BOUNCE_COOLDOWN_SECONDS)
            || recent(&times.kick, KICK_MASKS_BOUNCE_SECONDS)
        {
            continue;
        }

        // The hardest impact this step, and what it was against.
        let mut hardest: Option<(f32, Entity)> = None;
        for pair in collisions.collisions_with(ball) {
            let other = if pair.collider1 == ball { pair.collider2 } else { pair.collider1 };
            if cats.contains(other) {
                continue;
            }
            let impact = pair.manifolds.iter()
                .flat_map(|manifold| &manifold.points)
                .filter(|point| point.normal_impulse > 0.0)
                .map(|point| -point.normal_speed)
                .fold(0.0, f32::max);
            if impact >= BOUNCE_MIN_SPEED && hardest.is_none_or(|(best, _)| impact > best) {
                hardest = Some((impact, other));
            }
        }

        let Some((impact, other)) = hardest else { continue; };
        times.bounce.insert(ball, now);
        let surface = if goals.contains(other) {
            Surface::Post
        } else if layers.get(other).is_ok_and(|l| l.memberships.has_all(GameLayer::Ground)) {
            Surface::Ground
        } else {
            Surface::Wall
        };
        let strength = ((impact - BOUNCE_MIN_SPEED) / (BOUNCE_FULL_SPEED - BOUNCE_MIN_SPEED))
            .clamp(0.0, 1.0);
        let mut synth = thud(strength, surface, &mut rng);
        synth.volume *= times.duck(ball, now);
        sounds.write(PlaySound(synth));
    }
}

/// Linear interpolation across a `(min, max)` range.
fn lerp((min, max): (f32, f32), t: f32) -> f32 {
    min + (max - min) * t
}

/// Builds a ball thud from `THUD`. `strength` runs from 0 (softest) to 1 (hardest): harder hits
/// are louder, lower and longer.
pub fn thud(strength: f32, surface: Surface, rng: &mut Rng) -> Synth {
    let s = strength.clamp(0.0, 1.0);
    let (surface_pitch, brightness) = surface.colour();
    let pitch = lerp(PITCH_RANGE, s) * surface_pitch * (1.0 + rng.range_f32(-PITCH_JITTER, PITCH_JITTER));
    Synth {
        pitch: THUD.pitch * pitch,
        end_pitch: THUD.end_pitch * pitch,
        brightness,
        decay: THUD.decay * lerp(DECAY_RANGE, s),
        volume: THUD.volume * lerp(VOLUME_RANGE, s),
        seed: rng.u64(),
        ..THUD
    }
}
