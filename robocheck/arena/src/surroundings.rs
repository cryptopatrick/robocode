//! What robocode 0.5.0's `Observation` doesn't carry: how far the robot can
//! drive before it touches a wall or a building.

#[cfg(feature = "verus")]
use vstd::prelude::*;

use robocode::rules::ROBOT_RADIUS;
use robocode::terrain::Terrain;
use robocode::{Point, Rect};

/// Free distance, in whole pixels, along the heading (`ahead`) and the
/// reverse heading (`behind`) before the robot's body touches a wall or a
/// building. Passed to `decide` next to the `Observation`.
#[cfg_attr(feature = "verus", verus_verify)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Surroundings {
    pub clearance_ahead: i32,
    pub clearance_behind: i32,
}

/// The margin RoboCheck's starter rule keeps from walls and buildings, in pixels.
#[cfg_attr(feature = "verus", verus_verify)]
pub const OBSTACLE_MARGIN: i32 = 80;

#[cfg(feature = "verus")]
verus! {
/// Surroundings the arena can produce: clearances are never negative.
pub open spec fn surroundings_ok(env: Surroundings) -> bool {
    env.clearance_ahead >= 0 && env.clearance_behind >= 0
}
}

/// How far the centre at (`x`, `y`) can travel along `heading_degrees`
/// (0 = up, clockwise) before the body touches a wall or a building.
fn free_distance(
    terrain: &Terrain,
    width: f64,
    height: f64,
    x: f64,
    y: f64,
    heading_degrees: f64,
) -> f64 {
    let cap = (width * width + height * height).sqrt();
    let radians = heading_degrees.to_radians();
    let (dx, dy) = (radians.sin(), -radians.cos());
    // Walls: the field inset by the radius.
    let mut best = cap;
    let r = ROBOT_RADIUS;
    for (pos, d, lo, hi) in [(x, dx, r, width - r), (y, dy, r, height - r)] {
        if d > 1e-12 {
            best = best.min((hi - pos) / d);
        } else if d < -1e-12 {
            best = best.min((lo - pos) / d);
        }
    }
    let from = Point { x, y };
    let to = Point { x: x + dx * cap, y: y + dy * cap };
    for b in &terrain.buildings {
        let grown: Rect = b.grow(r);
        if let Some(t) = grown.first_contact(from, to) {
            best = best.min(t * cap);
        }
    }
    best.max(0.0)
}

/// The robot's [`Surroundings`] at (`x`, `y`) facing `heading_degrees`, in a
/// `width` × `height` field with `terrain`. Rounded down, so a rule that
/// keeps a margin holds for the true distance too.
pub fn clearance(
    terrain: &Terrain,
    width: f64,
    height: f64,
    x: f64,
    y: f64,
    heading_degrees: f64,
) -> Surroundings {
    let ahead = free_distance(terrain, width, height, x, y, heading_degrees);
    let behind =
        free_distance(terrain, width, height, x, y, heading_degrees + 180.0);
    Surroundings {
        clearance_ahead: ahead.floor() as i32,
        clearance_behind: behind.floor() as i32,
    }
}
