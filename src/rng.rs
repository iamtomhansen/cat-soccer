//! A tiny xorshift random number generator for gameplay randomness (not for security).

use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;

/// Seed used when the system clock is unavailable; any non-zero value works.
const FALLBACK_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
/// Number of mantissa bits in an `f32`, used to turn random bits into a unit float.
const F32_MANTISSA_BITS: u32 = 24;

/// Plugin that provides a time-seeded `Rng` resource.
pub struct RngPlugin;

impl Plugin for RngPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Rng::from_time());
    }
}

/// A fast xorshift64 generator shared through the world.
#[derive(Resource, Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// Creates a generator from a seed; a zero seed is replaced with a fixed non-zero one.
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 { FALLBACK_SEED } else { seed })
    }

    /// Creates a generator seeded from the system clock.
    pub fn from_time() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(FALLBACK_SEED, |d| d.as_nanos() as u64);
        Self::new(nanos)
    }

    /// Returns the next random 64-bit value.
    pub fn u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Returns a random float in `[0, 1)`.
    pub fn f32(&mut self) -> f32 {
        (self.u64() >> (u64::BITS - F32_MANTISSA_BITS)) as f32 / (1u64 << F32_MANTISSA_BITS) as f32
    }

    /// Returns a random float in `[min, max)`.
    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.f32() * (max - min)
    }

    /// Returns a random index in `[0, len)`; `len` must be non-zero.
    pub fn index(&mut self, len: usize) -> usize {
        (self.u64() % len as u64) as usize
    }
}
