//! Large, gentle light and dark variation across the whole field that never repeats.
//!
//! Stored as vertex colours on a subdivided ground plane, which are blended smoothly.

use bevy::prelude::*;

use super::noise::fractal_noise;

/// Tint in the darkest patches; multiplies the texture colour.
const DARK_TINT: LinearRgba = LinearRgba::rgb(0.50, 0.50, 0.50);
/// Tint in the lightest patches; multiplies the texture colour.
const LIGHT_TINT: LinearRgba = LinearRgba::rgb(1.10, 1.10, 1.10);
/// Noise cells across the field for the largest patches.
const PERIOD: u32 = 3;
/// Noise layers; each adds detail at twice the frequency and half the strength.
const OCTAVES: u32 = 3;
/// Seed for the noise, different from the texture's so the two patterns don't line up.
const SEED: u32 = 0x7f4a_7c15;

/// Tint at world position `xz` (x = world X, y = world Z) on a field `field_size` across.
pub fn field_tint(xz: Vec2, field_size: f32) -> LinearRgba {
    let uv = xz / field_size + 0.5;
    let amount = fractal_noise(uv.x, uv.y, PERIOD, OCTAVES, SEED);
    DARK_TINT.mix(&LIGHT_TINT, amount)
}

/// `color` multiplied by the tint at `xz`, the way the GPU combines vertex colour and texture.
pub fn tinted(color: LinearRgba, xz: Vec2, field_size: f32) -> LinearRgba {
    let tint = field_tint(xz, field_size);
    LinearRgba::new(color.red * tint.red, color.green * tint.green, color.blue * tint.blue, color.alpha)
}

/// Vertex colours for `mesh` from `field_tint`, one per vertex position.
pub fn tint_vertices(mesh: &mut Mesh, field_size: f32) {
    let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| a.as_float3())
    else {
        return;
    };
    let colors: Vec<[f32; 4]> = positions
        .iter()
        .map(|p| field_tint(Vec2::new(p[0], p[2]), field_size).to_f32_array())
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
}
