//! `SafeShooter`: the RoboVerus running example.
//!
//! It spins its radar, swings its gun toward the target it last saw, strafes
//! around it while keeping away from walls, and fires **only when it is safe**:
//!
//! > if `decide(obs).fire == Some(p)` then the energy is strictly more than `p`
//! > (firing everything would leave the robot at 0, which the engine kills), the gun is
//! > cool, a target was scanned this turn, and the gun is aimed at it.
//!
//! With the `verus` feature, Verus proves that property about [`decide`].

// This module is written in the subset of Rust that Verus verifies, which
// rules out some idioms clippy prefers (`clamp`, `RangeInclusive::contains`,
// match guards).
#![allow(
    clippy::manual_range_contains,
    clippy::manual_clamp,
    clippy::collapsible_match,
    clippy::single_match
)]

#[cfg(feature = "verus")]
use vstd::prelude::*;

use crate::strategy::{Action, Observation, Strategy};
use crate::units::{Power, angle_diff};
// Spec functions exist only while Verus runs, so their imports are gated on `verus_only`.
#[cfg(verus_only)]
use crate::units::{MAX_POWER, MIN_POWER, spec_angle_diff};

/// Largest gun/target misalignment, in decidegrees (3.0°), that counts as aimed.
#[cfg_attr(feature = "verus", verus_verify)]
pub const AIM_TOLERANCE: i32 = 30;

/// Distance from a wall, in pixels, at which SafeShooter stops driving toward it.
#[cfg_attr(feature = "verus", verus_verify)]
pub const WALL_MARGIN: i32 = 80;

/// Largest radar turn per turn (the engine allows 45°).
#[cfg_attr(feature = "verus", verus_verify)]
pub const MAX_RADAR_TURN: i32 = 450;

/// Largest gun turn requested per turn (the engine allows 20°).
#[cfg_attr(feature = "verus", verus_verify)]
pub const MAX_GUN_TURN: i32 = 200;

#[cfg(feature = "verus")]
verus! {

/// Observations the runner can produce: normalized angles and a robot inside
/// a reasonably sized battlefield.
pub open spec fn well_formed(obs: Observation) -> bool {
    &&& 0 <= obs.heading.0 < 3600
    &&& 0 <= obs.gun_heading.0 < 3600
    &&& 0 <= obs.radar_heading.0 < 3600
    &&& 0 <= obs.field_width <= 100_000
    &&& 0 <= obs.field_height <= 100_000
    &&& obs.target matches Some(t) ==> 0 <= t.bearing.0 < 3600
}

/// The gun points at the target within [`AIM_TOLERANCE`].
pub open spec fn aimed(obs: Observation) -> bool {
    obs.target matches Some(t)
        && -AIM_TOLERANCE <= spec_angle_diff(t.bearing.0 as int, obs.gun_heading.0 as int) <= AIM_TOLERANCE
}

/// Firing with power `p` is safe: the power is legal, and energy stays above
/// zero afterwards (the engine kills a robot whose energy reaches 0).
pub open spec fn safe_to_fire(obs: Observation, p: Power) -> bool {
    &&& MIN_POWER.0 <= p.0 <= MAX_POWER.0
    &&& obs.energy.0 > p.0
    &&& obs.gun_heat.0 == 0
    &&& obs.target matches Some(t) && t.age == 0
    &&& aimed(obs)
}

} // verus!

/// Whether the gun points at the target within [`AIM_TOLERANCE`].
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires well_formed(*obs),
    ensures r == aimed(*obs),
))]
pub fn is_aimed(obs: &Observation) -> bool {
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            -AIM_TOLERANCE <= d && d <= AIM_TOLERANCE
        }
        None => false,
    }
}

/// SafeShooter's decision for one turn.
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires well_formed(*obs),
    ensures r.fire matches Some(p) ==> safe_to_fire(*obs, p),
))]
pub fn decide(obs: &Observation) -> Action {
    let (ahead, turn_body) = movement(obs);
    let mut turn_gun = None;
    let mut fire = None;
    match obs.target {
        Some(t) => {
            if t.age <= 1 {
                let d = angle_diff(t.bearing, obs.gun_heading);
                turn_gun = Some(if d > MAX_GUN_TURN {
                    MAX_GUN_TURN
                } else if d < -MAX_GUN_TURN {
                    -MAX_GUN_TURN
                } else {
                    d
                });
                let p = if t.distance < 150 && obs.energy.0 > 3000 {
                    Power(3000)
                } else {
                    Power(1000)
                };
                if obs.gun_heat.0 == 0 && t.age == 0 && is_aimed(obs) && obs.energy.0 > p.0 {
                    fire = Some(p);
                }
            }
        }
        None => {}
    }
    Action { fire, ahead: Some(ahead), turn_body, turn_gun, turn_radar: Some(radar_turn(obs)) }
}

/// Radar lock: with a target scanned this turn, turn past it by as much again
/// so the next sweep crosses it; otherwise spin at full speed to find one.
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires well_formed(*obs),
    ensures -MAX_RADAR_TURN <= r <= MAX_RADAR_TURN,
))]
fn radar_turn(obs: &Observation) -> i32 {
    match obs.target {
        Some(t) => {
            if t.age == 0 {
                let overshoot = 2 * angle_diff(t.bearing, obs.radar_heading);
                if overshoot > MAX_RADAR_TURN {
                    MAX_RADAR_TURN
                } else if overshoot < -MAX_RADAR_TURN {
                    -MAX_RADAR_TURN
                } else {
                    overshoot
                }
            } else {
                MAX_RADAR_TURN
            }
        }
        None => MAX_RADAR_TURN,
    }
}

/// Whether travelling in direction `h` (decidegrees) heads into a wall the
/// robot is already within [`WALL_MARGIN`] of.
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(
    requires
        0 <= h < 3600,
        well_formed(*obs),
))]
fn toward_near_wall(obs: &Observation, h: i32) -> bool {
    let left = obs.x < WALL_MARGIN && h > 1800;
    let right = obs.x > obs.field_width - WALL_MARGIN && h > 0 && h < 1800;
    let top = obs.y < WALL_MARGIN && (h < 900 || h > 2700);
    let bottom = obs.y > obs.field_height - WALL_MARGIN && h > 900 && h < 2700;
    left || right || top || bottom
}

/// Strafe perpendicular to the target, driving forward or backward, whichever
/// does not head into a nearby wall. Returns `(ahead, turn_body)`.
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(
    requires well_formed(*obs),
))]
fn movement(obs: &Observation) -> (i32, Option<i32>) {
    let h = obs.heading.0;
    let back = if h >= 1800 { h - 1800 } else { h + 1800 };
    if !toward_near_wall(obs, h) {
        (100, strafe_turn(obs))
    } else if !toward_near_wall(obs, back) {
        (-100, strafe_turn(obs))
    } else {
        // Cornered: stop and turn a quarter so one direction opens up.
        (0, Some(900))
    }
}

/// Body turn that makes the robot perpendicular to the target (the nearer of
/// the two perpendiculars), or `None` with no target.
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(
    requires well_formed(*obs),
))]
fn strafe_turn(obs: &Observation) -> Option<i32> {
    match obs.target {
        Some(t) => {
            let b = t.bearing.0;
            let perp = if b >= 2700 { b - 2700 } else { b + 900 };
            let d = angle_diff(crate::units::Angle(perp), obs.heading);
            Some(if d > 900 { d - 1800 } else if d < -900 { d + 1800 } else { d })
        }
        None => None,
    }
}

/// The [`Strategy`] wrapper around [`decide`].
pub struct SafeShooter;

impl Strategy for SafeShooter {
    fn name(&self) -> &str {
        "SafeShooter"
    }
    fn decide(&mut self, obs: &Observation) -> Action {
        decide(obs)
    }
}
