use std::time::Duration;

use bevy::prelude::*;
use touchy::prelude::*;

/// Placeholder for now: A plugin to manage the cat's animations.
pub struct CatAnimationPlugin;

impl Plugin for CatAnimationPlugin {
    fn build(&self, _app: &mut App) {
        app.init_resource::<CatAnimationAssets>()
            .add_systems(Startup, load_cat_animation_graph)
            .add_systems(Update, (
                link_animation_player,
                update_cat_animations,
            ));
    }
}

/// The animation graph and nodes for the cat.
#[derive(Resource)]
pub struct CatAnimations {
    pub graph_handle: Handle<AnimationGraph>,
    pub walk: AnimationNodeIndex,
    pub idle: AnimationNodeIndex,
    pub ear_r: AnimationNodeIndex,
    pub ear_l: AnimationNodeIndex,
    pub tail_swish: AnimationNodeIndex,
    pub tail_twitch: AnimationNodeIndex,
}

/// Dynamic playback & blend state attached to the logical Cat entity
#[derive(Component)]
pub struct CatLocomotion {
    pub is_walking: bool,
    pub idle_weight: f32,
    pub walk_weight: f32,
    pub twitch_timer: Timer,
}

impl Default for CatLocomotion {
    fn default() -> Self {
        Self {
            is_walking: false,
            idle_weight: 1.0,
            walk_weight: 0.0,
            twitch_timer: Timer::from_seconds(10.0, TimerMode::Repeating),
        }
    }
}

fn load_cat_animation_graph(
    mut anims: ResMut<CatAnimationAssets>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    asset_server: Res<AssetServer>,
) {
    let mut graph = AnimationGraph::new();

    // Load clips by their GLTF animation index
    let walk_clip = asset_server.load(GltfAssetLabel::Animation(0).from_asset("models/cat.glb"));
    let idle_clip = asset_server.load(GltfAssetLabel::Animation(1).from_asset("models/cat.glb"));
    let ear_r_clip = asset_server.load(GltfAssetLabel::Animation(2).from_asset("models/cat.glb"));
    let ear_l_clip = asset_server.load(GltfAssetLabel::Animation(3).from_asset("models/cat.glb"));
    let tail_swish_clip = asset_server.load(GltfAssetLabel::Animation(4).from_asset("models/cat.glb"));
    let tail_twitch_clip = asset_server.load(GltfAssetLabel::Animation(5).from_asset("models/cat.glb"));

    anims.walk = graph.add_clip(walk_clip, 1.0, graph.root);
    anims.idle = graph.add_clip(idle_clip, 1.0, graph.root);
    anims.ear_r = graph.add_clip(ear_r_clip, 1.0, graph.root);
    anims.ear_l = graph.add_clip(ear_l_clip, 1.0, graph.root);
    anims.tail_swish = graph.add_clip(tail_swish_clip, 1.0, graph.root);
    anims.tail_twitch = graph.add_clip(tail_twitch_clip, 1.0, graph.root);

    anims.graph = graphs.add(graph);
}

/// The AnimationPlayer is used to play animations for a scene. Give it a handle to the
/// AnimationGraph.
fn link_animation_player(
    mut commands: Commands,
    anims: Res<CatAnimationAssets>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
) {
    for (entity, mut player) in &mut players {
        // Link the graph to this player
        commands.entity(entity).insert(AnimationGraphHandle(anims.graph.clone()));

        // Start both idle and walk; blend using weights
        player.play(anims.idle).repeat().set_weight(1.0);
        player.play(anims.walk).repeat().set_weight(0.0);
    }
}

/// A system that updates the cat's animations based on its movement and random events.
fn update_cat_animations(
    time: Res<Time>,
    anims: Res<CatAnimationAssets>,
    mut cats: Query<&mut CatLocomotion, With<Cat>>,
    mut players: Query<&mut AnimationPlayer>,
) {
    let Ok(mut loco) = cats.single_mut() else { return };
    let dt = time.delta_secs();

    // 1. Smoothly interpolate weights (250ms crossfade)
    let (target_idle, target_walk) = if loco.is_walking {
        (0.0, 1.0)
    } else {
        (1.0, 0.0)
    };

    loco.idle_weight = move_towards(loco.idle_weight, target_idle, 4.0 * dt);
    loco.walk_weight = move_towards(loco.walk_weight, target_walk, 4.0 * dt);

    for mut player in &mut players {
        // Apply blend weights
        if let Some(idle) = player.animation_mut(anims.idle) {
            idle.set_weight(loco.idle_weight);
        }
        if let Some(walk) = player.animation_mut(anims.walk) {
            walk.set_weight(loco.walk_weight);
        }

        // 2. Ambient fidgets (twitches and swishes)
        loco.twitch_timer.tick(time.delta());
        if loco.twitch_timer.just_finished() {
            let choice = fastrand::u8(0..4);
            let action_node = match choice {
                0 => anims.ear_r,
                1 => anims.ear_l,
                2 => anims.tail_swish,
                _ => anims.tail_twitch,
            };

            // Start plays once from the beginning without clearing the base loops
            player.start(action_node);

            // Re-arm between 8 and 14 seconds
            let next_delay = 8.0 + fastrand::f32() * 6.0;
            loco.twitch_timer.set_duration(Duration::from_secs_f32(next_delay));
        }
    }
}

fn move_towards(current: f32, target: f32, max_step: f32) -> f32 {
    if (target - current).abs() <= max_step {
        target
    } else {
        current + (target - current).signum() * max_step
    }
}