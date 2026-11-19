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
        r.fire matches Some(p) ==> (((p.0 as int) >= 100) && ((p.0 as int) <= 3000) && ((obs.energy.0 as int) > (p.0 as int))), // S1
))]
pub fn decide(obs: &Observation, around: &Surroundings) -> Action {
let mut turn_gun = None; let mut turn_body = None; let mut turn_radar = Some(MAX_RADAR_TURN); match obs.target { Some(t) => { turn_gun = Some(clamp_to(angle_diff(t.bearing, obs.gun_heading), MAX_GUN_TURN)); turn_radar = Some(clamp_to(2 * angle_diff(t.bearing, obs.radar_heading), MAX_RADAR_TURN)); turn_body = Some(angle_diff(t.bearing, obs.heading)); } None => {} } let e = obs.energy.0; let mut fire = None; if obs.gun_heat.0 == 0 && e > 100 { let p = if e > 3000 { 3000 } else { e - 1 }; fire = Some(Power(p)); } Action { fire, ahead: Some(100), turn_body, turn_gun, turn_radar }
}

// ---- candidate helpers below (untrusted) ----
fn clamp_to(v: i32, m: i32) -> i32 { if v > m { m } else if v < -m { -m } else { v } }
