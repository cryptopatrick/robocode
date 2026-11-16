//! TRUSTED TEMPLATE (robocheck). The candidate supplies only the body of
//! `decide` and helper functions below the marked line; the reference spec on
//! `decide` is written by robocheck (Phase 0's human-written reference, or
//! rendered from AI-formalised rules) and may not be changed by the candidate.
#![allow(
    clippy::manual_range_contains,
    clippy::manual_clamp,
    clippy::collapsible_match,
    clippy::single_match,
    unused_imports,
    dead_code
)]

#[cfg(feature = "verus")]
use vstd::prelude::*;

use robocheck_arena::{OBSTACLE_MARGIN, Surroundings};
use robocode::robots::safe_shooter::{AIM_TOLERANCE, MAX_GUN_TURN, MAX_RADAR_TURN, WALL_MARGIN};
use robocode::strategy::{Action, Observation};
use robocode::units::{Angle, Power, angle_diff};
#[cfg(verus_only)]
use robocheck_arena::surroundings_ok;
#[cfg(verus_only)]
use robocode::robots::safe_shooter::{aimed, heading_into_near_wall, ready, reverse_heading};
#[cfg(verus_only)]
use robocode::strategy::well_formed;
#[cfg(verus_only)]
use robocode::units::{MAX_POWER, MIN_POWER, spec_angle_diff};

/// The candidate's decision for one turn, checked against the spec. `around`
/// is what robocode 0.5.0's `Observation` lacks: the free distance to walls
/// and buildings ahead and behind (robocheck-arena).
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires well_formed(*obs), surroundings_ok(*around),
    ensures
        r.fire matches Some(p) ==> ((obs.target matches Some(t) && t.age == 0) && aimed(*obs)), // S2
))]
pub fn decide(obs: &Observation, around: &Surroundings) -> Action {
    let (ahead0, turn_body) = movement(obs);
    let ahead = if ahead0 > 0 && around.clearance_ahead < ahead0 {
        0
    } else if ahead0 < 0 && around.clearance_behind < -ahead0 {
        0
    } else {
        ahead0
    };
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

// ---- candidate helpers below (untrusted) ----
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires well_formed(*obs),
    ensures r == aimed(*obs),
))]
fn is_aimed(obs: &Observation) -> bool {
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            -AIM_TOLERANCE <= d && d <= AIM_TOLERANCE
        }
        None => false,
    }
}

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

#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires
        0 <= h < 3600,
        well_formed(*obs),
    ensures r == heading_into_near_wall(*obs, h as int),
))]
fn toward_near_wall(obs: &Observation, h: i32) -> bool {
    let left = obs.x < WALL_MARGIN && h > 1800;
    let right = obs.x > obs.field_width - WALL_MARGIN && h > 0 && h < 1800;
    let top = obs.y < WALL_MARGIN && (h < 900 || h > 2700);
    let bottom = obs.y > obs.field_height - WALL_MARGIN && h > 900 && h < 2700;
    left || right || top || bottom
}

#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires well_formed(*obs),
    ensures
        r.0 > 0 ==> !heading_into_near_wall(*obs, obs.heading.0 as int),
        r.0 < 0 ==> !heading_into_near_wall(*obs, reverse_heading(obs.heading.0 as int)),
))]
fn movement(obs: &Observation) -> (i32, Option<i32>) {
    let h = obs.heading.0;
    let back = if h >= 1800 { h - 1800 } else { h + 1800 };
    if !toward_near_wall(obs, h) {
        (100, strafe_turn(obs))
    } else if !toward_near_wall(obs, back) {
        (-100, strafe_turn(obs))
    } else {
        (0, Some(900))
    }
}

#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(
    requires well_formed(*obs),
))]
fn strafe_turn(obs: &Observation) -> Option<i32> {
    match obs.target {
        Some(t) => {
            let b = t.bearing.0;
            let perp = if b >= 2700 { b - 2700 } else { b + 900 };
            let d = angle_diff(Angle(perp), obs.heading);
            Some(if d > 900 { d - 1800 } else if d < -900 { d + 1800 } else { d })
        }
        None => None,
    }
}
