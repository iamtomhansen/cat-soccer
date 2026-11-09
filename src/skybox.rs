//! A gradient skybox, generated at startup and attached to every 3d camera.

use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::Skybox;
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};

/// Width and height of each cubemap face, in pixels.
const FACE_SIZE: u32 = 128;
/// Number of faces in a cubemap.
const FACE_COUNT: u32 = 6;
/// Sky colour straight overhead.
const ZENITH_COLOR: Color = Color::srgb(0.10, 0.35, 0.85);
/// Sky colour at the horizon.
const HORIZON_COLOR: Color = Color::srgb(0.75, 0.90, 0.95);
/// Colour of the ground far below the horizon.
const GROUND_COLOR: Color = Color::srgb(0.30, 0.30, 0.25);
/// Exponent shaping the sky gradient; below 1 keeps the pale band thin near the horizon.
const SKY_FALLOFF: f32 = 0.2;
/// How far below the horizon (as a sine of the angle) the ground blends to full colour.
const GROUND_BLEND: f32 = 0.02;
/// Skybox brightness in cd/m², chosen to look right with the default camera exposure.
const SKY_BRIGHTNESS: f32 = 1000.0;

/// Plugin that builds the sky cubemap and attaches it to cameras as they spawn.
pub struct SkyboxPlugin;

impl Plugin for SkyboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, build_sky_cubemap)
            .add_systems(Update, attach_skybox);
    }
}

/// Handle to the generated sky cubemap.
#[derive(Resource)]
pub struct SkyCubemap(pub Handle<Image>);

/// Generates the gradient cubemap image and stores its handle.
fn build_sky_cubemap(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let face_pixels = (FACE_SIZE * FACE_SIZE) as usize;
    let mut data = Vec::with_capacity(face_pixels * FACE_COUNT as usize * 4);

    for face in 0..FACE_COUNT {
        for y in 0..FACE_SIZE {
            for x in 0..FACE_SIZE {
                let s = (x as f32 + 0.5) / FACE_SIZE as f32 * 2.0 - 1.0;
                let t = (y as f32 + 0.5) / FACE_SIZE as f32 * 2.0 - 1.0;
                let dir = face_direction(face, s, t);
                data.extend_from_slice(&sky_color(dir).to_srgba().to_u8_array());
            }
        }
    }

    let mut image = Image::new(
        Extent3d { width: FACE_SIZE, height: FACE_SIZE, depth_or_array_layers: FACE_COUNT },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });

    commands.insert_resource(SkyCubemap(images.add(image)));
}

/// Adds the skybox to any 3d camera that does not have one yet.
fn attach_skybox(
    mut commands: Commands,
    sky: Res<SkyCubemap>,
    cameras: Query<Entity, (With<Camera3d>, Without<Skybox>)>,
) {
    for camera in &cameras {
        commands.entity(camera).insert(Skybox {
            image: Some(sky.0.clone()),
            brightness: SKY_BRIGHTNESS,
            ..default()
        });
    }
}

/// World direction through texel (`s`, `t`) of a cubemap face, both in -1..1 with `t` pointing down.
fn face_direction(face: u32, s: f32, t: f32) -> Vec3 {
    match face {
        0 => Vec3::new(1.0, -t, -s),
        1 => Vec3::new(-1.0, -t, s),
        2 => Vec3::new(s, 1.0, t),
        3 => Vec3::new(s, -1.0, -t),
        4 => Vec3::new(s, -t, 1.0),
        _ => Vec3::new(-s, -t, -1.0),
    }
    .normalize()
}

/// Sky colour seen looking along `dir`: a horizon-to-zenith gradient above, ground below.
fn sky_color(dir: Vec3) -> Color {
    if dir.y >= 0.0 {
        HORIZON_COLOR.mix(&ZENITH_COLOR, dir.y.powf(SKY_FALLOFF))
    } else {
        HORIZON_COLOR.mix(&GROUND_COLOR, (-dir.y / GROUND_BLEND).min(1.0))
    }
}
