//! The visible ground: a tiled procedural grass texture, a gentle tint across the whole field
//! that hides the tiling, and optional 3d grass blades.
//!
//! A test bed for comparing ground techniques. M switches the texture's mipmaps on and off (look
//! at the far field while the camera moves to see the difference), and G shows or hides the blades.

mod blades;
mod noise;
mod texture;
mod tint;

use bevy::light::NotShadowCaster;
use bevy::math::Affine2;
use bevy::prelude::*;

use crate::arena::{ArenaBounds, GROUND_SIZE};
use blades::blades_mesh;
use texture::grass_image;
use tint::tint_vertices;

/// Times the grass texture repeats across the ground in each direction.
const TEXTURE_REPEATS: f32 = 6.0;
/// Cuts across the ground plane in each direction, giving the vertices to tint.
const GROUND_SUBDIVISIONS: u32 = 31;
/// Whether the texture starts with its mipmaps on (M toggles).
const MIPMAPS_ON_START: bool = true;
/// Most samples the GPU takes along a stretched texture footprint; 1 turns anisotropic filtering off.
const ANISOTROPY: u16 = 16;
/// Ground roughness: 1 is fully matte, which suits grass.
const GROUND_ROUGHNESS: f32 = 0.95;
/// Ground reflectance at head-on angles; low, so the ground doesn't look wet.
const GROUND_REFLECTANCE: f32 = 0.2;
/// Number of 3d grass blades; 0 skips building them.
const BLADE_COUNT: usize = 1_000;
/// Whether the blades start visible (G toggles).
const BLADES_ON_START: bool = true;
/// Whether blades cast shadows. Each shadow-casting blade is drawn again into the shadow map.
const BLADES_CAST_SHADOWS: bool = false;

/// Plugin that spawns the textured ground and grass blades, with keys to compare techniques.
pub struct GroundPlugin;

impl Plugin for GroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_ground);
    }
}

/// The ground's material and its two texture versions, for switching between them.
#[derive(Resource)]
struct GroundTextures {
    /// The ground plane's material.
    material: Handle<StandardMaterial>,
    /// Grass texture with a full mipmap chain.
    with_mipmaps: Handle<Image>,
    /// Grass texture with only its full-size level.
    without_mipmaps: Handle<Image>,
}

/// Marker for the grass blades mesh.
#[derive(Component)]
struct GrassBlades;

/// Spawns the textured ground plane and, if `BLADE_COUNT` is above 0, the grass blades.
fn spawn_ground(
    mut commands: Commands,
    bounds: Res<ArenaBounds>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let with_mipmaps = images.add(grass_image(true, ANISOTROPY));
    let without_mipmaps = images.add(grass_image(false, ANISOTROPY));
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(
            if MIPMAPS_ON_START { with_mipmaps.clone() } else { without_mipmaps.clone() },
        ),
        perceptual_roughness: GROUND_ROUGHNESS,
        reflectance: GROUND_REFLECTANCE,
        // Scaling texture coordinates past 1 makes the Repeat sampler tile the texture.
        uv_transform: Affine2::from_scale(Vec2::splat(TEXTURE_REPEATS)),
        ..default()
    });
    let mut ground_mesh =
        Plane3d::default().mesh().size(GROUND_SIZE, GROUND_SIZE).subdivisions(GROUND_SUBDIVISIONS).build();
    tint_vertices(&mut ground_mesh, GROUND_SIZE);
    commands.spawn((
        Name::new("Ground visual"),
        Mesh3d(meshes.add(ground_mesh)),
        MeshMaterial3d(material.clone()),
        Transform::default(),
    ));
    commands.insert_resource(GroundTextures { material, with_mipmaps, without_mipmaps });

    if BLADE_COUNT == 0 {
        return;
    }
    let blade_material = materials.add(StandardMaterial {
        // White, so the per-vertex colours show as they are.
        base_color: Color::WHITE,
        perceptual_roughness: GROUND_ROUGHNESS,
        reflectance: GROUND_REFLECTANCE,
        // Blades are single triangles, so draw both sides. `double_sided` stays off: it would flip
        // the up-pointing normals on back faces to point down, turning half the blades black.
        cull_mode: None,
        ..default()
    });
    let mut blades = commands.spawn((
        GrassBlades,
        Name::new("Grass blades"),
        Mesh3d(meshes.add(blades_mesh(BLADE_COUNT, &bounds, GROUND_SIZE, TEXTURE_REPEATS))),
        MeshMaterial3d(blade_material),
        Transform::default(),
        if BLADES_ON_START { Visibility::Inherited } else { Visibility::Hidden },
    ));
    if !BLADES_CAST_SHADOWS {
        blades.insert(NotShadowCaster);
    }
}