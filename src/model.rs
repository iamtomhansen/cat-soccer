//! Handles GLB model loading and scaling to desired dimensions.

use bevy::prelude::*;
use bevy::camera::primitives::Aabb;

/// Plugin for 3D model size normalization and scaling.
pub struct ModelNormalizePlugin;

impl Plugin for ModelNormalizePlugin {
    fn build(&self, app: &mut App) {
        // In post-update, Aabb components and meshes are populated.
        app.add_systems(Update, spawn_model_visuals)
            .add_systems(PostUpdate, apply_model_normalization);
    }
}

/// Declarative configuration for a 3D model asset.
#[derive(Component, Clone, Debug)]
pub struct ModelConfig {
    /// Handle to the GLB model asset.
    pub handle: Handle<WorldAsset>,
    /// Desired size of the model in world-space (set "size").
    pub size: ModelSize,
    /// Rotation to orient the asset into desired world-space orientation (set "forward").
    pub rotation_offset: Quat,
    /// Translation to moved the asset into desired world-space position (set "center").
    pub translation_offset: Vec3,
}

impl ModelConfig {
    /// Creates a new model configuration with the given asset handle and size.
    pub fn new(handle: Handle<WorldAsset>, size: ModelSize) -> Self {
        Self {
            handle,
            size,
            rotation_offset: Quat::IDENTITY,
            translation_offset: Vec3::ZERO,
        }
    }

    /// Convenience method to set the rotation offset in degrees around the Y (vertical) axis.
    pub fn rotation_degrees(mut self, rotation: f32) -> Self {
        self.rotation_offset = Quat::from_rotation_y(rotation.to_radians());
        self
    }
}

/// Spawns the child visual enetity for any newly added ModelConfig component.
fn spawn_model_visuals(
    mut commands: Commands,
    query: Query<(Entity, &ModelConfig), Added<ModelConfig>>,
) {
    for (root_entity, config) in &query {
        commands.entity(root_entity).with_children(|parent| {
            parent.spawn((
                WorldAssetRoot(config.handle.clone()),
                Transform {
                    translation: config.translation_offset,
                    rotation: config.rotation_offset,
                    ..default()
                },
            ));
        });
    }
}

/// Declarative sizing rule for any spawned 3D scene/model root.
#[derive(Component, Debug, Clone, Copy)]
pub enum ModelSize {
    /// For the height to match this world-space value.
    Height(f32),
    /// Scales uniformly until the longest axis matches this value.
    MaxDimension(f32),
    /// Scales uniformly until it fits within this box (x,y,z).
    FitBox(Vec3),
}

/// Marker added to the root once it is normalized so it is skipped by future updates.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ModelNormalized;

fn apply_model_normalization(
    mut commands: Commands,
    aabb_query: Query<(&Aabb, &ChildOf), Added<Aabb>>,
    parents: Query<&ChildOf>,
    mut roots: Query<(Entity, &mut Transform, &ModelConfig), Without<ModelNormalized>>,
) {
    for (aabb, child_of) in &aabb_query {
        // Walk up hierarchy to find entity with ModelSize component.
        let mut current = child_of.parent();
        let target_root = loop {
            if roots.contains(current) {
                break Some(current);
            }
            if let Ok(next_parent) = parents.get(current) {
                current = next_parent.parent();
            } else {
                break None;
            }
        };

        let Some(root_entity) = target_root else { continue };

        if let Ok((_, mut transform, config)) = roots.get_mut(root_entity) {
            let dimensions = aabb.half_extents.to_vec3() * 2.0;

            let uniform_scale = match config.size {
                ModelSize::Height(target_height) => {
                    if dimensions.y > 0.0001 {
                        target_height / dimensions.y
                    } else {
                        1.0
                    }
                }
                ModelSize::MaxDimension(target_max) => {
                    // Find longest axis
                    let max_dim = dimensions.x.max(dimensions.y).max(dimensions.z);
                    if max_dim > 0.0001 {
                        target_max / max_dim
                    } else {
                        1.0
                    }
                }
                ModelSize::FitBox(target_box) => {
                    let scale_x = target_box.x / dimensions.x.max(0.0001);
                    let scale_y = target_box.y / dimensions.y.max(0.0001);
                    let scale_z = target_box.z / dimensions.z.max(0.0001);
                    scale_x.min(scale_y).min(scale_z)
                }
            };

            transform.scale = Vec3::splat(uniform_scale);
            commands.entity(root_entity).insert(ModelNormalized);
        }
    }
}