//! The soccer ball's mesh, painted procedurally.

use bevy::prelude::*;

/// Subdivisions of the ball's icosphere mesh.
const BALL_SUBDIVISIONS: u32 = 5;
/// Colour of the ball's panels.
const PANEL_COLOR: Color = Color::WHITE;
/// Colour of the ball's twelve pentagon patches.
const PATCH_COLOR: Color = Color::srgb(0.05, 0.05, 0.05);
/// Angular radius of each pentagon patch, in radians.
const PATCH_ANGLE: f32 = 0.33;

/// An icosphere of `radius` with black pentagon patches painted in vertex colours, like a soccer ball.
pub fn ball_mesh(radius: f32) -> Mesh {
    let mut mesh = Sphere::new(radius)
        .mesh()
        .ico(BALL_SUBDIVISIONS)
        .expect("ball subdivisions are within the icosphere limit");

    // The twelve pentagons of a soccer ball sit on the vertices of an icosahedron.
    let phi = (1.0 + 5.0_f32.sqrt()) * 0.5;
    let mut patches = Vec::with_capacity(12);
    for a in [-1.0, 1.0] {
        for b in [-phi, phi] {
            patches.push(Vec3::new(0.0, a, b).normalize());
            patches.push(Vec3::new(a, b, 0.0).normalize());
            patches.push(Vec3::new(b, 0.0, a).normalize());
        }
    }

    let min_dot = PATCH_ANGLE.cos();
    let panel = PANEL_COLOR.to_linear().to_f32_array();
    let patch = PATCH_COLOR.to_linear().to_f32_array();
    let colors: Vec<[f32; 4]> = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(positions) => positions
            .as_float3()
            .unwrap_or_default()
            .iter()
            .map(|&p| {
                let dir = Vec3::from(p).normalize_or_zero();
                if patches.iter().any(|c| c.dot(dir) > min_dot) { patch } else { panel }
            })
            .collect(),
        None => Vec::new(),
    };
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh
}