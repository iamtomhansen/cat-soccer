//! Builds animation graphs for spawned glTF models from named clips.

use std::collections::HashMap;

use bevy::animation::graph::AnimationMask;
use bevy::animation::AnimationTargetId;
use bevy::gltf::Gltf;
use bevy::prelude::*;

/// Number of mask groups an `AnimationMask` can hold, so the most bone-restricted clips per model.
const MAX_MASKED_CLIPS: usize = AnimationMask::BITS as usize;

/// Plugin that gives spawned models an animation graph built from their `AnimationConfig`.
pub struct ModelAnimationPlugin;

impl Plugin for ModelAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (mark_new_players, build_model_animations).chain());
    }
}

/// Declares the named glTF clips a model plays; put it next to `ModelConfig` on the model root.
#[derive(Component, Clone, Debug)]
pub struct AnimationConfig {
    /// Handle to the glTF asset that holds the clips.
    pub gltf: Handle<Gltf>,
    /// The clips to add to the model's animation graph.
    pub clips: Vec<ClipConfig>,
}

/// One named clip, optionally restricted to some bones.
#[derive(Clone, Debug)]
pub struct ClipConfig {
    /// Name of the animation in the glTF file.
    pub name: String,
    /// Bones (with their descendants) the clip may animate; empty means the whole rig.
    pub bones: Vec<String>,
}

impl AnimationConfig {
    /// Creates a config with no clips for the given glTF asset.
    pub fn new(gltf: Handle<Gltf>) -> Self {
        Self { gltf, clips: Vec::new() }
    }

    /// Adds a clip that animates the whole rig.
    pub fn clip(self, name: &str) -> Self {
        self.masked_clip(name, &[])
    }

    /// Adds a clip that only animates the given bones and their descendants.
    pub fn masked_clip(mut self, name: &str, bones: &[&str]) -> Self {
        self.clips.push(ClipConfig {
            name: name.to_owned(),
            bones: bones.iter().map(|bone| (*bone).to_owned()).collect(),
        });
        self
    }
}

/// A clip that made it into a model's animation graph.
#[derive(Clone, Copy, Debug)]
pub struct ClipInfo {
    /// The clip's node in the animation graph.
    pub node: AnimationNodeIndex,
    /// Length of the clip in seconds.
    pub duration: f32,
}

/// Inserted on the model root once its animation graph is built and linked.
#[derive(Component, Clone, Debug)]
pub struct ModelAnimations {
    /// The entity holding the model's `AnimationPlayer`.
    pub player: Entity,
    /// Loaded clips, by glTF animation name.
    pub clips: HashMap<String, ClipInfo>,
}

impl ModelAnimations {
    /// Looks up a loaded clip by its glTF name.
    pub fn clip(&self, name: &str) -> Option<ClipInfo> {
        self.clips.get(name).copied()
    }
}

/// Marks an animation player that still needs its graph built.
#[derive(Component)]
#[component(storage = "SparseSet")]
struct PendingAnimationLink;

/// Flags every newly spawned animation player for linking.
fn mark_new_players(mut commands: Commands, players: Query<Entity, Added<AnimationPlayer>>) {
    for player in &players {
        commands.entity(player).insert(PendingAnimationLink);
    }
}

/// Builds and links an animation graph for each pending player once its glTF has loaded.
fn build_model_animations(
    mut commands: Commands,
    players: Query<Entity, With<PendingAnimationLink>>,
    ancestors: Query<&ChildOf>,
    configs: Query<&AnimationConfig>,
    children: Query<&Children>,
    bones: Query<(Option<&Name>, Option<&AnimationTargetId>)>,
    gltfs: Res<Assets<Gltf>>,
    clips: Res<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for player in &players {
        // The model root is the nearest ancestor carrying an AnimationConfig.
        let Some(root) = ancestors.iter_ancestors(player).find(|e| configs.contains(*e)) else {
            commands.entity(player).remove::<PendingAnimationLink>();
            continue;
        };
        let config = configs.get(root).expect("root was found by its config");

        // Wait until the glTF and every clip it holds are loaded.
        let Some(gltf) = gltfs.get(&config.gltf) else { continue };
        if gltf.animations.iter().any(|clip| clips.get(clip).is_none()) {
            continue;
        }

        let rig = Rig::collect(player, &children, &bones);
        let mut graph = AnimationGraph::new();
        let mut loaded = HashMap::new();
        let mut mask_group = 0;

        for clip_config in &config.clips {
            let Some(handle) = gltf.named_animations.get(clip_config.name.as_str()) else {
                let mut names: Vec<_> = gltf.named_animations.keys().collect();
                names.sort();
                warn!("No animation named {:?}; available: {:?}", clip_config.name, names);
                continue;
            };
            let duration = clips.get(handle).map_or(0.0, AnimationClip::duration);
            let node = graph.add_clip(handle.clone(), 1.0, graph.root);

            if !clip_config.bones.is_empty() {
                if mask_group >= MAX_MASKED_CLIPS {
                    warn!("Too many bone-restricted clips; {:?} animates every bone", clip_config.name);
                } else {
                    // Every target outside the allowed bones joins this clip's mask group.
                    let allowed = rig.targets_under(&clip_config.bones, &children, &bones);
                    for target in rig.targets.iter().filter(|t| !allowed.contains(t)) {
                        graph.add_target_to_mask_group(*target, mask_group as u32);
                    }
                    graph[node].add_mask_group(mask_group as u32);
                    mask_group += 1;
                }
            }
            loaded.insert(clip_config.name.clone(), ClipInfo { node, duration });
        }

        commands
            .entity(player)
            .remove::<PendingAnimationLink>()
            .insert(AnimationGraphHandle(graphs.add(graph)));
        commands.entity(root).insert(ModelAnimations { player, clips: loaded });
    }
}

/// The named bones and animation targets under one animation player.
struct Rig {
    /// Bone entities by name.
    named: HashMap<String, Entity>,
    /// Every animation target in the rig.
    targets: Vec<AnimationTargetId>,
}

impl Rig {
    /// Gathers bone names and targets from the player and its descendants.
    fn collect(
        player: Entity,
        children: &Query<&Children>,
        bones: &Query<(Option<&Name>, Option<&AnimationTargetId>)>,
    ) -> Self {
        let mut named = HashMap::new();
        let mut targets = Vec::new();
        for entity in std::iter::once(player).chain(children.iter_descendants(player)) {
            let Ok((name, target)) = bones.get(entity) else { continue };
            if let Some(name) = name {
                named.insert(name.as_str().to_owned(), entity);
            }
            targets.extend(target.copied());
        }
        Self { named, targets }
    }

    /// Returns the targets of the given bones and all their descendants.
    fn targets_under(
        &self,
        bone_names: &[String],
        children: &Query<&Children>,
        bones: &Query<(Option<&Name>, Option<&AnimationTargetId>)>,
    ) -> Vec<AnimationTargetId> {
        let mut found = Vec::new();
        for bone_name in bone_names {
            let Some(&bone) = self.named.get(bone_name) else {
                warn!("No bone named {bone_name:?} to restrict a clip to");
                continue;
            };
            for entity in std::iter::once(bone).chain(children.iter_descendants(bone)) {
                if let Ok((_, Some(target))) = bones.get(entity) {
                    found.push(*target);
                }
            }
        }
        found
    }
}
