//! Logic to make a cat move around.

use std::time::Duration;

use bevy::prelude::*;
use touchy::prelude::*;

/// The clock speed of the cat.
const CAT_SPEED: f32 = 2.0;
const WELCOME_WIDTH: f32 = 400.0;
const WELCOME_TITLE: &str = "Cat!";
const WELCOME: &[&str] = &["This is a cat.", "Press to make it walk around."];

/// The plugin that sets up and runs the game.
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (
                setup,
                setup_animations,
            ).chain())
            .add_systems(Update, (
                camera_follow,
                move_cat,
            ));
    }
}

/// A marker component for the cat entity.
#[derive(Component)]
struct Cat;

/// A component to track current animation state of the cat.
#[derive(Component)]
pub struct CatAnimationState {
    /// Timer for random ear/tail twitches.
    pub twitch_timer: Timer,
    /// Whether the cat is currently walking.
    pub is_walking: bool,
}

/// Initialize the game world.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>, layers: Res<UiLayers>) {
    // Spawn the cat, camera, and light.
    commands.spawn((
        Cat,
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/cat.glb"))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        FollowCamera {
            offset: Vec3::new(0.0, 2.0, 4.5),
            smooth_speed: 6.0,
        },
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_4,
            -std::f32::consts::FRAC_PI_4,
            0.0,
        )),
    ));

    // Spawn the UI elements on their appropriate layers.
    commands.entity(layers.screen).with_children(|screen| {
        screen
            .spawn(Node {
                position_type: PositionType::Absolute,
                left: px(24.0),
                bottom: px(24.0),
                width: px(180.0),
                height: px(180.0),
                ..default()
            })
            .with_children(|area| {
                let config = JoystickConfig::new()
                    .floating(true)
                    .radius(72.0);
                area.joystick(config);
            });
    });

    commands
        .spawn((Welcome, UiLayer::Overlay, Column, Card))
        .node(|node| node.width = px(WELCOME_WIDTH))
        .with_children(|card| {
            card.heading(WELCOME_TITLE);
            for line in WELCOME {
                card.text(*line);
            }
            card.row(|row| {
                row.spacer();
                row.button(ButtonConfig::new("Meow"))
                    .observe(close_welcome);
            });
        });
}

/// System to move the cat based on joystick input.
fn move_cat(
    time: Res<Time>,
    sticks: Query<&JoystickValue>,
    mut cats: Query<&mut Transform, With<Cat>>,
) {
    let Ok(stick) = sticks.single() else { return; };
    if stick.0.length_squared() < 0.001 {
        return;
    }

    // Map 2D joystick (x, y) to 3D ground plane (x, -y -> z)
    let move_dir = Vec3::new(stick.0.x, 0.0, -stick.0.y).normalize();

    for mut transform in &mut cats {
        // Translate
        transform.translation += move_dir * CAT_SPEED * time.delta_secs();

        // Smoothly rotate cat towards heading
        let target_rot = Transform::default().looking_to(move_dir, Vec3::Y).rotation;
        transform.rotation = transform.rotation.slerp(target_rot, 10.0 * time.delta_secs());
    }
}

/// Marker component for the welcome overlay.
#[derive(Component)]
struct Welcome;

/// Dismiss the welcome overlay. Listens for the `Activate` event of the "Meow" button.
fn close_welcome(_: On<Activate>, welcome: Query<Entity, With<Welcome>>, mut commands: Commands) {
    for entity in &welcome {
        commands.entity(entity).despawn();
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

fn setup_animations(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let mut graph = AnimationGraph::new();

    // Map your Blender NLA / action names directly
    let walk_clip = asset_server.load(GltfAssetLabel::Animation(0).from_asset("models/cat.glb"));
    let idle_clip = asset_server.load(GltfAssetLabel::Animation(1).from_asset("models/cat.glb"));
    let ear_r_clip = asset_server.load(GltfAssetLabel::Animation(2).from_asset("models/cat.glb"));
    let ear_l_clip = asset_server.load(GltfAssetLabel::Animation(3).from_asset("models/cat.glb"));
    let tail_swish_clip = asset_server.load(GltfAssetLabel::Animation(4).from_asset("models/cat.glb"));
    let tail_twitch_clip = asset_server.load(GltfAssetLabel::Animation(5).from_asset("models/cat.glb"));

    // Register nodes in the graph
    let walk = graph.add_clip(walk_clip, 1.0, graph.root);
    let idle = graph.add_clip(idle_clip, 1.0, graph.root);
    let ear_r = graph.add_clip(ear_r_clip, 1.0, graph.root);
    let ear_l = graph.add_clip(ear_l_clip, 1.0, graph.root);
    let tail_swish = graph.add_clip(tail_swish_clip, 1.0, graph.root);
    let tail_twitch = graph.add_clip(tail_twitch_clip, 1.0, graph.root);

    let graph_handle = graphs.add(graph);

    commands.insert_resource(CatAnimations {
        graph_handle,
        walk,
        idle,
        ear_r,
        ear_l,
        tail_swish,
        tail_twitch,
    });
}

/// The AnimationPlayer is used to play animations for a scene. Give it a handle to the
/// AnimationGraph.
fn link_animations(
    mut commands: Commands,
    animations: Res<CatAnimations>,
    query: Query<Entity, Added<AnimationPlayer>>,
) {
    for entity in &query {
        commands.entity(entity).insert(AnimationGraphHandle(animations.graph_handle.clone()));
    }
}

/// A system that updates the cat's animations based on its movement and random events.
fn update_cat_animations(
    time: Res<Time>,
    anims: Res<CatAnimations>,
    sticks: Query<&JoystickValue>,
    mut anim_state: Query<&mut CatAnimationState>,
    mut players: Query<&mut AnimationPlayer>,
) {
    let Ok(mut state) = anim_state.single_mut() else { return; };
    let stick_mag = sticks.single().map(|s| s.0.length()).unwrap_or(0.0);
    let walking = stick_mag > 0.05;

    for mut player in &mut players {
        // 1. Transition between Idle and Walk
        if walking && !state.is_walking {
            state.is_walking = true;
            player
                .play(anims.walk)
                .repeat()
                .set_speed(1.2)
                .set_transition_duration(Duration::from_millis(250));
        } else if !walking && state.is_walking {
            state.is_walking = false;
            player
                .play(anims.idle)
                .repeat()
                .set_transition_duration(Duration::from_millis(300));
        }

        // Ensure idle starts playing on frame 1
        if !player.is_playing_animation(anims.idle) && !state.is_walking {
            player.play(anims.idle).repeat();
        }

        // 2. Randomly trigger one-shot fusions (Ear Twitch / Tail Swish)
        // state.twitch_timer.tick(time.delta());
        // if state.twitch_timer.just_finished() {
        //     // Randomly pick one of the 4 overlay actions
        //     let choice = fastrand::u8(0..4);
        //     let action_node = match choice {
        //         0 => anims.ear_r,
        //         1 => anims.ear_l,
        //         2 => anims.tail_swish,
        //         _ => anims.tail_twitch,
        //     };

        //     // Start plays once from beginning and lets the underlying idle/walk keep running
        //     player.start(action_node);

        //     // Re-arm timer with random interval between 8 and 14 seconds
        //     let next_secs = 8.0 + fastrand::f32() * 6.0;
        //     state.twitch_timer.set_duration(Duration::from_secs_f32(next_secs));
        // }
    }
}