//! A small soccer goal: two posts and a crossbar. When the ball goes through, the goal poofs and
//! reappears at a random spot and angle in the arena.

mod effects;
mod mesh;
mod sound;

use std::f32::consts::{FRAC_1_SQRT_2, TAU};

use avian3d::prelude::*;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;

use crate::arena::ArenaBounds;
use crate::ball::{Ball, BALL_RADIUS};
use crate::cat::Cat;
use crate::rng::Rng;
use effects::GoalEffectsPlugin;
use mesh::goal_bars;
use sound::GoalSoundPlugin;

/// Width of the goal opening between the posts, in world units (5/3 of the ball's diameter).
const GOAL_WIDTH: f32 = 0.4;
/// Height of the goal opening under the crossbar, in world units.
const GOAL_HEIGHT: f32 = 0.2;
/// Thickness of the posts and the crossbar.
const GOAL_BAR_THICKNESS: f32 = 0.02;
/// Colour of the posts and the crossbar.
const GOAL_COLOR: Color = Color::WHITE;
/// Surface roughness of the posts and the crossbar.
const GOAL_ROUGHNESS: f32 = 0.4;
/// Bounciness of the posts and the crossbar. Wins over what hits them.
const GOAL_RESTITUTION: f32 = 0.95;
/// Where the goal first stands on the ground (x = world X, y = world Z).
const INITIAL_GOAL_POS: Vec2 = Vec2::new(1.0, 1.0);
/// The goal's first turn about the vertical axis, in radians.
const INITIAL_GOAL_ROT: f32 = 0.0;
/// Gap kept between each post and the nearest wall when the goal moves, in ball diameters.
const WALL_CLEARANCE_BALLS: f32 = 1.1;
/// Closest the goal's centre may reappear to the ball or the cat, so it never lands on them.
const KEEP_AWAY_DISTANCE: f32 = 0.4;
/// Random spots tried before giving up and leaving the goal where it is.
const PLACEMENT_ATTEMPTS: u32 = 100;

/// Plugin that spawns the goal, detects goals scored, and moves the goal after each one, with
/// effects and sounds.
pub struct GoalPlugin;

impl Plugin for GoalPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((GoalEffectsPlugin, GoalSoundPlugin))
            .add_message::<GoalScored>()
            .add_message::<GoalMoved>()
            .add_systems(Startup, spawn_goal)
            .add_systems(Update, (detect_goals, relocate_goals).chain());
    }
}

/// Marker for a goal. Its origin is on the ground in the middle of the opening.
#[derive(Component)]
pub struct Goal;

/// The goal's visible bars, a child of the goal so effects can scale them without the collider.
#[derive(Component)]
struct GoalVisual;

/// Sent when a ball passes through a goal.
#[derive(Message, Clone, Copy, Debug)]
pub struct GoalScored {
    /// The goal the ball went through.
    pub goal: Entity,
    /// The ball that went through.
    pub ball: Entity,
    /// Where the ball's centre crossed the goal line, in world space.
    pub point: Vec3,
}

/// Sent when a goal jumps to a new spot.
#[derive(Message, Clone, Copy, Debug)]
pub struct GoalMoved {
    /// The goal that moved.
    pub goal: Entity,
    /// Where the goal was.
    pub from: Transform,
    /// Where the goal is now.
    pub to: Transform,
}

/// Spawns the goal as a static body, with its bars as child meshes.
fn spawn_goal(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let bars = goal_bars(GOAL_WIDTH, GOAL_HEIGHT, GOAL_BAR_THICKNESS);
    let material = materials.add(StandardMaterial {
        base_color: GOAL_COLOR,
        perceptual_roughness: GOAL_ROUGHNESS,
        ..default()
    });
    let collider = Collider::compound(
        bars.iter()
            .map(|bar| (bar.offset, Quat::IDENTITY, Collider::cuboid(bar.size.x, bar.size.y, bar.size.z)))
            .collect(),
    );

    commands
        .spawn((
            Goal,
            Name::new("Goal"),
            Transform::from_xyz(INITIAL_GOAL_POS.x, 0.0, INITIAL_GOAL_POS.y)
                .with_rotation(Quat::from_rotation_y(INITIAL_GOAL_ROT)),
            Visibility::default(),
            RigidBody::Static,
            collider,
            Restitution::new(GOAL_RESTITUTION).with_combine_rule(CoefficientCombine::Max),
        ))
        .with_children(|goal| {
            goal.spawn((GoalVisual, Transform::default(), Visibility::default()))
                .with_children(|visual| {
                    for bar in &bars {
                        visual.spawn((
                            Mesh3d(meshes.add(bar.cuboid())),
                            MeshMaterial3d(material.clone()),
                            Transform::from_translation(bar.offset),
                        ));
                    }
                });
        });
}

/// Sends `GoalScored` when a ball's centre crosses a goal's opening, from either side.
///
/// Checks the path between frames rather than the current position, so a fast ball that skips
/// over the goal line in one frame still counts.
fn detect_goals(
    goals: Query<(Entity, &Transform), With<Goal>>,
    balls: Query<(Entity, &Transform), With<Ball>>,
    mut last_positions: Local<EntityHashMap<Vec3>>,
    mut scored: MessageWriter<GoalScored>,
) {
    last_positions.retain(|ball, _| balls.contains(*ball));
    for (ball, ball_transform) in &balls {
        let now = ball_transform.translation;
        let Some(before) = last_positions.insert(ball, now) else { continue; };
        for (goal, goal_transform) in &goals {
            let to_local = goal_transform.compute_affine().inverse();
            let (a, b) = (to_local.transform_point3(before), to_local.transform_point3(now));
            // The goal line is the goal's local XY plane; only a move across it can score.
            if (a.z < 0.0) == (b.z < 0.0) {
                continue;
            }
            let crossing = a.lerp(b, a.z / (a.z - b.z));
            if crossing.x.abs() < GOAL_WIDTH * 0.5 && crossing.y < GOAL_HEIGHT {
                scored.write(GoalScored { goal, ball, point: goal_transform.transform_point(crossing) });
            }
        }
    }
}

/// Moves each goal that was scored on to a random free spot, and sends `GoalMoved`.
fn relocate_goals(
    mut scored: MessageReader<GoalScored>,
    mut goals: Query<&mut Transform, With<Goal>>,
    obstacles: Query<&Transform, (Or<(With<Ball>, With<Cat>)>, Without<Goal>)>,
    bounds: Res<ArenaBounds>,
    mut rng: ResMut<Rng>,
    mut moved: MessageWriter<GoalMoved>,
) {
    let mut handled = Vec::new();
    for score in scored.read() {
        info!("Goal! Ball {} went through goal {} at {}", score.ball, score.goal, score.point);
        if handled.contains(&score.goal) {
            continue;
        }
        handled.push(score.goal);
        let Ok(mut transform) = goals.get_mut(score.goal) else { continue; };
        let avoid: Vec<Vec2> = obstacles.iter().map(|t| t.translation.xz()).collect();
        let Some(to) = random_placement(&bounds, &avoid, &mut rng) else {
            warn!("Found no free spot for goal {}; leaving it where it is", score.goal);
            continue;
        };
        let from = *transform;
        *transform = to;
        moved.write(GoalMoved { goal: score.goal, from, to });
    }
}

/// A random spot and turn for a goal that keeps both posts clear of the walls and the goal's
/// centre clear of every point in `avoid`, or `None` if no try found one.
fn random_placement(bounds: &ArenaBounds, avoid: &[Vec2], rng: &mut Rng) -> Option<Transform> {
    // Posts are square, so their corners reach this far out from their centres.
    let post_reach = GOAL_BAR_THICKNESS * FRAC_1_SQRT_2;
    let min_wall_distance = BALL_RADIUS * 2.0 * WALL_CLEARANCE_BALLS + post_reach;
    let post_x = (GOAL_WIDTH + GOAL_BAR_THICKNESS) * 0.5;
    let (min, max) = bounds.min_max();
    (0..PLACEMENT_ATTEMPTS).find_map(|_| {
        let center = Vec2::new(rng.range_f32(min.x, max.x), rng.range_f32(min.y, max.y));
        if avoid.iter().any(|p| p.distance(center) < KEEP_AWAY_DISTANCE) {
            return None;
        }
        let transform = Transform::from_xyz(center.x, 0.0, center.y)
            .with_rotation(Quat::from_rotation_y(rng.range_f32(0.0, TAU)));
        let posts_clear = [-post_x, post_x].into_iter().all(|x| {
            let post = transform.transform_point(Vec3::X * x).xz();
            bounds.wall_distance(post).is_some_and(|distance| distance >= min_wall_distance)
        });
        posts_clear.then_some(transform)
    })
}
