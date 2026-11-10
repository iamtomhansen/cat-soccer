//! Scattered 3d grass blades: thin triangles merged into one mesh, so they draw in one call.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::texture::GrassPattern;
use super::tint::tinted;
use crate::arena::ArenaBounds;
use crate::rng::Rng;

/// Width of a blade at its base.
const BLADE_WIDTH: f32 = 0.006;
/// Shortest blade height (the cat is 0.25 tall).
const BLADE_MIN_HEIGHT: f32 = 0.015;
/// Tallest blade height.
const BLADE_MAX_HEIGHT: f32 = 0.035;
/// Most a blade's tip leans sideways, as a fraction of its height.
const BLADE_MAX_LEAN: f32 = 0.4;
/// How much gradient there is between a blade's base and tip (0 = from black, 1 = all equal).
const COLOR_GRADIENT: f32 = 0.8;
/// How much brighter a blade's tip is than the ground under it (1 = no change).
const COLOR_BRIGHTNESS: f32 = 1.3;
/// Seed for blade placement, so the field looks the same every run.
const SEED: u64 = 0x5eed_b1ad;

/// Builds one mesh holding `count` blades scattered over the arena floor.
///
/// Each blade takes its colour from the ground under it (the texture, which tiles
/// `texture_repeats` times across `ground_size`, times the field tint), so blades and ground match.
pub fn blades_mesh(count: usize, bounds: &ArenaBounds, ground_size: f32, texture_repeats: f32) -> Mesh {
    let mut rng = Rng::new(SEED);
    let (min, max) = bounds.min_max();
    let mut positions = Vec::with_capacity(count * 3);
    let mut normals = Vec::with_capacity(count * 3);
    let mut colors = Vec::with_capacity(count * 3);

    while positions.len() < count * 3 {
        let spot = Vec2::new(rng.range_f32(min.x, max.x), rng.range_f32(min.y, max.y));
        if bounds.wall_distance(spot).is_none() {
            continue;
        }
        let height = rng.range_f32(BLADE_MIN_HEIGHT, BLADE_MAX_HEIGHT);
        let facing = Vec2::from_angle(rng.range_f32(0.0, std::f32::consts::TAU));
        let lean = Vec2::from_angle(rng.range_f32(0.0, std::f32::consts::TAU))
            * rng.range_f32(0.0, BLADE_MAX_LEAN * height);
        let half_base = Vec3::new(facing.x, 0.0, facing.y) * BLADE_WIDTH * 0.5;
        let base = Vec3::new(spot.x, 0.0, spot.y);

        positions.push((base - half_base).to_array());
        positions.push((base + half_base).to_array());
        positions.push((base + Vec3::new(lean.x, height, lean.y)).to_array());

        // Point every normal straight up, so blades light like the ground (no flickering).
        normals.extend([[0.0, 1.0, 0.0]; 3]);

        let uv = (spot / ground_size + 0.5) * texture_repeats;
        let ground = tinted(GrassPattern::color(uv.x, uv.y), spot, ground_size);
        let base_color = (ground * COLOR_GRADIENT).with_alpha(1.0).to_f32_array();
        let tip_color = (ground * COLOR_BRIGHTNESS).with_alpha(1.0).to_f32_array();
        colors.extend([base_color, base_color, tip_color]);
    }

    let indices = (0..positions.len() as u32).collect();
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_indices(Indices::U32(indices))
}
