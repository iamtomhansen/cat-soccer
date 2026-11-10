//! A tiny procedural synth. A sound is a sine tone mixed with noise, fading out together. The tone
//! slides from a starting pitch to an end pitch as it fades, which is what makes a thud go "dum".

use std::f32::consts::TAU;
use std::num::NonZero;
use std::time::Duration;

use bevy::audio::{ChannelCount, Decodable, SampleRate, Source};
use bevy::prelude::*;

/// Samples per second the synth renders at.
const SAMPLE_RATE: u32 = 44_100;
/// Fade level at which a sound counts as finished (-60 dB).
const SILENCE: f32 = 0.001;
/// Lowest brightness used, so the noise never smooths away to nothing.
const MIN_BRIGHTNESS: f32 = 0.01;
/// Noise seed used when a sound's seed is zero; any non-zero value works.
const FALLBACK_NOISE_SEED: u64 = 0x2545_F491_4F6C_DD1D;

/// A procedurally generated sound effect, playable with `AudioPlayer<Synth>`.
#[derive(Asset, TypePath, Clone, Copy, Debug)]
pub struct Synth {
    /// Pitch of the tone when the sound starts, in Hz.
    pub pitch: f32,
    /// Pitch the tone slides to as the sound fades, in Hz. Same as `pitch` for a steady note.
    pub end_pitch: f32,
    /// Loudness of the tone, from 0 (none) upwards.
    pub tone: f32,
    /// Loudness of the noise, from 0 (none) upwards.
    pub noise: f32,
    /// Brightness of the noise, from 0 (dull rumble) to 1 (bright hiss).
    pub brightness: f32,
    /// How quickly the sound fades, in seconds: it drops to about a third every `decay`.
    pub decay: f32,
    /// Silence before the sound starts, in seconds.
    pub delay: f32,
    /// Overall loudness.
    pub volume: f32,
    /// Seed of the noise, so each play of the same sound can differ.
    pub seed: u64,
}

impl Default for Synth {
    fn default() -> Self {
        Self {
            pitch: 440.0,
            end_pitch: 440.0,
            tone: 1.0,
            noise: 0.0,
            brightness: 0.5,
            decay: 0.1,
            delay: 0.0,
            volume: 1.0,
            seed: 0,
        }
    }
}

impl Synth {
    /// Length of the sound in seconds, until it has faded to silence.
    pub fn duration(&self) -> f32 {
        self.delay + self.decay * (1.0 / SILENCE).ln()
    }
}

impl Decodable for Synth {
    type Decoder = Voice;

    fn decoder(&self) -> Voice {
        Voice {
            synth: *self,
            index: 0,
            len: (self.duration() * SAMPLE_RATE as f32).ceil() as usize,
            phase: 0.0,
            smoothed_noise: 0.0,
            rng: if self.seed == 0 { FALLBACK_NOISE_SEED } else { self.seed },
        }
    }
}

/// A `Synth` being rendered, one sample at a time.
pub struct Voice {
    /// The sound being rendered.
    synth: Synth,
    /// Index of the next sample.
    index: usize,
    /// Total number of samples.
    len: usize,
    /// Position in the tone's sine wave, in radians.
    phase: f32,
    /// The noise after smoothing, which is how `brightness` dulls it.
    smoothed_noise: f32,
    /// Noise generator state (xorshift64).
    rng: u64,
}

impl Voice {
    /// Next white noise sample in `[-1, 1)`.
    fn white(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / (1u64 << 23) as f32 - 1.0
    }
}

impl Iterator for Voice {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.index >= self.len {
            return None;
        }
        let s = self.synth;
        let t = self.index as f32 / SAMPLE_RATE as f32 - s.delay;
        self.index += 1;
        if t < 0.0 {
            return Some(0.0);
        }

        // Fade: 1 at the start, falling towards 0.
        let fade = (-t / s.decay).exp();

        // Tone: a sine wave whose pitch slides from `pitch` to `end_pitch` as the sound fades.
        let pitch = s.end_pitch + (s.pitch - s.end_pitch) * fade;
        self.phase = (self.phase + TAU * pitch / SAMPLE_RATE as f32) % TAU;
        let tone = self.phase.sin();

        // Noise: random samples, smoothed (low-passed) more the lower the brightness.
        let white = self.white();
        self.smoothed_noise += (white - self.smoothed_noise) * s.brightness.clamp(MIN_BRIGHTNESS, 1.0);
        let noise = self.smoothed_noise;

        Some((s.tone * tone + s.noise * noise) * fade * s.volume)
    }
}

impl Source for Voice {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.len - self.index)
    }

    fn channels(&self) -> ChannelCount {
        NonZero::<u16>::MIN
    }

    fn sample_rate(&self) -> SampleRate {
        NonZero::new(SAMPLE_RATE).expect("sample rate is non-zero")
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(self.len as f32 / SAMPLE_RATE as f32))
    }
}
