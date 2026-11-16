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
        true,
))]
pub fn decide(obs: &Observation, around: &Surroundings) -> Action {
let mut turn_gun = None;
    let mut turn_body = None;
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(if d > MAX_GUN_TURN { MAX_GUN_TURN } else if d < -MAX_GUN_TURN { -MAX_GUN_TURN } else { d });
            turn_body = Some(angle_diff(t.bearing, obs.heading));
        }
        None => {}
    }
    let fire = if obs.gun_heat.0 == 0 { Some(Power(3000)) } else { None };
    Action { fire, ahead: Some(100), turn_body, turn_gun, turn_radar: Some(MAX_RADAR_TURN) }
}

// ---- candidate helpers below (untrusted) ----

