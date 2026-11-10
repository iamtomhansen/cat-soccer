//! The goal's shape: two posts and a crossbar, shared by its meshes and its collider.

use bevy::prelude::*;

/// One straight bar of the goal.
#[derive(Clone, Copy, Debug)]
pub struct GoalBar {
    /// Full size of the bar along each axis.
    pub size: Vec3,
    /// Centre of the bar relative to the goal's origin.
    pub offset: Vec3,
}

impl GoalBar {
    /// The bar as a Bevy cuboid primitive, centred on the origin.
    pub fn cuboid(&self) -> Cuboid {
        Cuboid::from_size(self.size)
    }
}

/// The two posts and the crossbar of a goal whose opening is `width` wide and `height` tall.
///
/// The goal's origin is on the ground at the middle of the opening, which lies in the XY plane,
/// so the ball goes through along Z. Bars are `thickness` thick and sit outside the opening.
pub fn goal_bars(width: f32, height: f32, thickness: f32) -> [GoalBar; 3] {
    let post_x = (width + thickness) * 0.5;
    let post = |x: f32| GoalBar {
        size: Vec3::new(thickness, height, thickness),
        offset: Vec3::new(x, height * 0.5, 0.0),
    };
    let crossbar = GoalBar {
        size: Vec3::new(width + thickness * 2.0, thickness, thickness),
        offset: Vec3::new(0.0, height + thickness * 0.5, 0.0),
    };
    [post(-post_x), post(post_x), crossbar]
}
