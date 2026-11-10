//! Seamless value noise and hashing shared by the grass texture and the field tint.

use bevy::prelude::*;

/// Sum of `octaves` layers of `value_noise`, each twice as fine and half as strong, in 0..1.
pub fn fractal_noise(u: f32, v: f32, period: u32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut weight, mut amplitude) = (0.0, 0.0, 1.0);
    for octave in 0..octaves {
        sum += value_noise(u, v, period << octave, seed.wrapping_add(octave)) * amplitude;
        weight += amplitude;
        amplitude *= 0.5;
    }
    sum / weight
}

/// Smooth random values on a `period` x `period` grid over 0..1, in 0..1.
///
/// Grid corners wrap at `period`, so the noise (and anything built from it) tiles without seams.
pub fn value_noise(u: f32, v: f32, period: u32, seed: u32) -> f32 {
    let (x, y) = (u.rem_euclid(1.0) * period as f32, v.rem_euclid(1.0) * period as f32);
    let (x0, y0) = (x.floor() as u32 % period, y.floor() as u32 % period);
    let (x1, y1) = ((x0 + 1) % period, (y0 + 1) % period);
    let (fx, fy) = (smooth(x.fract()), smooth(y.fract()));
    let top = hash(x0, y0, seed).lerp(hash(x1, y0, seed), fx);
    let bottom = hash(x0, y1, seed).lerp(hash(x1, y1, seed), fx);
    top.lerp(bottom, fy)
}

/// Smoothstep easing, so noise has no visible creases along grid lines.
fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// A repeatable pseudo-random value in 0..1 for integer coordinates.
pub fn hash(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = x.wrapping_mul(0x8da6_b343) ^ y.wrapping_mul(0xd816_3841) ^ seed;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h >> 8) as f32 / (1u32 << 24) as f32
}
