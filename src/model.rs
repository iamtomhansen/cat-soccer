//! Handles GLB model loading and scaling to desired dimensions.

use bevy::camera::primitives::{MeshAabb};
use bevy::math::Affine3A;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;

/// Dimensions smaller than this are treated as zero to avoid dividing by nothing.
const MIN_DIMENSION: f32 = 0.0001;

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

/// Bind-pose bounds of the scaled model in the root's local space, added on normalization.
#[derive(Component, Clone, Copy, Debug)]
pub struct ModelBounds {
    /// Centre of the bounds relative to the root.
    pub center: Vec3,
    /// Half the size of the bounds along each axis.
    pub half_extents: Vec3,
}


/// Scales each model to its `ModelSize` once its meshes have loaded, and records its `ModelBounds`.
///
/// Works in three steps:
///   1. Measure: find the box around every mesh in the model, in the root entity's space.
///   2. Scale: pick one scale factor from `ModelSize` and apply it to the visual child.
///   3. Record: store the scaled box as `ModelBounds`, which colliders are fitted to.
/// 
/// Two complications in measuring the box:
/// - It measures the mesh assets, which hold the model's rest pose, rather than the `Aabb`
///   component, because Bevy moves the `Aabb` of an animated (skinned) mesh along with its bones.
/// - A mesh's vertices normally sit relative to the mesh's own entity, so its box is moved up through
///   each parent's transform. Skinned meshes are the exception: glTF says their vertices sit relative
///   to the model's origin, and their entity's transform is ignored. In cat.glb the mesh entity is a
///   child of an `Armature` raised about half a cat, so counting it lifted the box off the ground.
fn apply_model_normalization(
    mut commands: Commands,
    roots: Query<(Entity, &ModelConfig, &Children), Without<ModelNormalized>>,
    visuals: Query<(), With<WorldAssetRoot>>,
    skinned: Query<(), With<SkinnedMesh>>,
    children: Query<&Children>,
    mut nodes: Query<(&mut Transform, Option<&Mesh3d>, Option<&ChildOf>), Without<ModelConfig>>,
    meshes: Res<Assets<Mesh>>,
) {
    for (root_entity, config, root_children) in &roots {
        // The visual child holds the glTF scene, its offset and its scale. Other children
        // (lights, cameras, effects) are not measured as the model.
        let Some(visual) = root_children.iter().find(|child| {
            visuals.contains(*child)
        }) else {
            continue;
        };

        // 1. Measure: grow a box around every mesh, in the root's space, at scale 1.
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        let mut waiting = false;

        let descendants = root_children
            .iter()
            .flat_map(|child| std::iter::once(child).chain(children.iter_descendants(child)));
        for entity in descendants {
            let Ok((_, Some(mesh3d), _)) = nodes.get(entity) else { continue };
            let Some(aabb) = meshes.get(mesh3d).and_then(MeshAabb::get_aabb) else {
                waiting = true;
                break;
            };
            // Transform that carries this mesh's vertices into the root's space. Skinned vertices
            // are relative to the model origin (the visual child), so start the chain there.
            let to_root = if skinned.contains(entity) {
                root_from_entity(visual, root_entity, &nodes)
            } else {
                root_from_entity(entity, root_entity, &nodes)
            };
            // Carry over all 8 corners of the mesh's box: a rotated box needs every corner.
            let (center, half) = (Vec3::from(aabb.center), Vec3::from(aabb.half_extents));
            for corner in 0..8 {
                let sign = Vec3::new(
                    if corner & 1 == 0 { -1.0 } else { 1.0 },
                    if corner & 2 == 0 { -1.0 } else { 1.0 },
                    if corner & 4 == 0 { -1.0 } else { 1.0 },
                );
                let point = to_root.transform_point3(center + half * sign);
                min = min.min(point);
                max = max.max(point);
            }
        }
        // Nothing to measure until the scene has spawned and its meshes have loaded.
        if waiting || min.x > max.x {
            continue;
        }

        // 2. Scale: one uniform factor, so the model keeps its proportions.
        let dimensions = max - min;
        let uniform_scale = match config.size {
            ModelSize::Height(target_height) => target_height / dimensions.y.max(MIN_DIMENSION),
            ModelSize::MaxDimension(target_max) => {
                target_max / dimensions.max_element().max(MIN_DIMENSION)
            }
            ModelSize::FitBox(target_box) => {
                (target_box / dimensions.max(Vec3::splat(MIN_DIMENSION))).min_element()
            }
        };

        let Some(visual) = root_children.iter().find(|child| visuals.contains(*child)) else {
            continue;
        };
        if let Ok((mut visual_transform, _, _)) = nodes.get_mut(visual) {
            visual_transform.scale = Vec3::splat(uniform_scale);
        }
        
        // 3. Record: the box was measured at scale 1, so scale it too. Scaling the visual child
        // shrinks everything towards that child's position (the configured offset), so the box
        // centre moves towards the offset, not towards the root.
        let offset = config.translation_offset;
        commands.entity(root_entity).insert((
            ModelNormalized,
            ModelBounds {
                center: offset + ((min + max) * 0.5 - offset) * uniform_scale,
                half_extents: dimensions * 0.5 * uniform_scale,
            },
        ));
    }
}

/// The transform from `entity`'s space to `root`'s space: every local transform from `entity`
/// up to, but not including, `root`, multiplied together.
fn root_from_entity(
    entity: Entity,
    root: Entity,
    nodes: &Query<(&mut Transform, Option<&Mesh3d>, Option<&ChildOf>), Without<ModelConfig>>,
) -> Affine3A {
    let mut affine = Affine3A::IDENTITY;
    let mut current = entity;
    while current != root {
        let Ok((local, _, parent)) = nodes.get(current) else { break };
        affine = local.compute_affine() * affine;
        let Some(parent) = parent else { break };
        current = parent.parent();
    }
    affine
}
