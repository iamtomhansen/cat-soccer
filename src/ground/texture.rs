//! A procedural grass texture that tiles seamlessly, with a hand-built mipmap chain.
//!
//! A mipmap chain is the same image stored again at half size, quarter size, and so on down to
//! 1x1, each level averaging 2x2 pixels of the one above. When the ground is far away or seen at
//! a low angle, one screen pixel covers many texels; the GPU then reads a smaller level, whose
//! texels are already the average of that area, instead of picking one texel at random (which
//! is what makes a detailed texture shimmer as the camera moves).

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::noise::{hash, value_noise};

/// Width and height of the texture in pixels. Must be a power of two so each level halves evenly.
pub const TEXTURE_SIZE: u32 = 256;
/// Average grass colour. Large light and dark patches come from `tint.rs`, not the tile, because
/// big shapes are what make a repeating tile easy to spot.
const GRASS_COLOR: Color = Color::srgb(0.175, 0.35, 0.11);
/// Noise cells across the texture for clumps of blades, the finest structure before single pixels.
const CLUMP_PERIOD: u32 = 64;
/// How much clumps brighten or darken the grass, as a fraction of its colour.
const CLUMP_STRENGTH: f32 = 0.35;
/// How much each pixel's random speckle brightens or darkens it, as a fraction of its colour.
const SPECKLE_STRENGTH: f32 = 0.25;
/// Seed for the noise, so the pattern is the same every run.
const SEED: u32 = 0x9e37_79b9;

/// Tileable grass pattern, sampled at texture coordinates in 0..1.
pub struct GrassPattern;

impl GrassPattern {
    /// Grass colour at (`u`, `v`). Wraps seamlessly at the edges.
    pub fn color(u: f32, v: f32) -> LinearRgba {
        // Small scale colour variation, simulating "clumps" of grass.
        let clumps = value_noise(u, v, CLUMP_PERIOD, SEED ^ 0x5678) * 2.0 - 1.0;
        GRASS_COLOR.to_linear() * (1.0 + clumps * CLUMP_STRENGTH)
    }

    /// Grass colour of texel (`x`, `y`) of a `TEXTURE_SIZE` texture, with per-pixel speckle.
    fn texel(x: u32, y: u32) -> LinearRgba {
        // Center of a pixel (x,y) in (u,v) 0..1 coordinates.
        let u = (x as f32 + 0.5) / TEXTURE_SIZE as f32;
        let v = (y as f32 + 0.5) / TEXTURE_SIZE as f32;
        // Add random speckle to grass surface.
        let speckle = hash(x, y, SEED ^ 0x9abc) * 2.0 - 1.0;
        let color = Self::color(u, v) * (1.0 + speckle * SPECKLE_STRENGTH);
        color.with_alpha(1.0)
    }
}

/// Builds the grass texture, with a full mipmap chain if `mipmaps` is set.
///
/// `anisotropy` is the most samples the GPU may take along a stretched footprint (1 turns it off).
pub fn grass_image(mipmaps: bool, anisotropy: u16) -> Image {
    // Level 0, in linear colour so averaging below is physically correct.
    let mut level: Vec<LinearRgba> = (0..TEXTURE_SIZE * TEXTURE_SIZE)
        .map(|i| GrassPattern::texel(i % TEXTURE_SIZE, i / TEXTURE_SIZE))
        .collect();
    let mut size = TEXTURE_SIZE;
    let mut data = to_srgb_bytes(&level);
    let mut level_count = 1;

    // Each further level averages 2x2 texels of the previous one, until the image is 1x1.
    while mipmaps && size > 1 {
        level = downsample(&level, size);
        size /= 2;
        data.extend(to_srgb_bytes(&level));
        level_count += 1;
    }

    let mut image = Image::new_uninit(
        Extent3d { width: TEXTURE_SIZE, height: TEXTURE_SIZE, depth_or_array_layers: 1 },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    // All levels go in one buffer, largest first; the descriptor says how many there are.
    image.data = Some(data);
    image.texture_descriptor.mip_level_count = level_count;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        // Repeat, so texture coordinates past 1 tile the texture across the ground.
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        // Linear everywhere: blend neighbouring texels, and blend between neighbouring levels
        // (trilinear filtering) so there is no visible line where one level hands over to the next.
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: anisotropy.max(1),
        ..default()
    });
    image
}

/// Used for mipmap generation.
/// 
/// Halves a square `size` x `size` image by averaging each 2x2 block. Averages colours in linear
/// space to avoid darkening the result.
fn downsample(pixels: &[LinearRgba], size: u32) -> Vec<LinearRgba> {
    let half = size / 2;
    let at = |x: u32, y: u32| pixels[(y * size + x) as usize];
    (0..half * half)
        .map(|i| {
            let (x, y) = ((i % half) * 2, (i / half) * 2);
            (at(x, y) + at(x + 1, y) + at(x, y + 1) + at(x + 1, y + 1)) * 0.25
        })
        .collect()
}

/// Converts linear colours to sRGB RGBA bytes, the layout of `Rgba8UnormSrgb`.
fn to_srgb_bytes(pixels: &[LinearRgba]) -> Vec<u8> {
    pixels.iter().flat_map(|p| Color::from(*p).to_srgba().to_u8_array()).collect()
}
