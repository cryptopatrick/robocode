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
        r.fire matches Some(p) ==> ((obs.target matches Some(t) && t.age == 0) && aimed(*obs)), // S2
))]
pub fn decide(obs: &Observation, around: &Surroundings) -> Action {
let mut fire = None;
    let mut turn_gun = None;
    let mut turn_body = None;
    let mut turn_radar = Some(MAX_RADAR_TURN);
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(clamp_turn(d, MAX_GUN_TURN));
            turn_body = Some(angle_diff(t.bearing, obs.heading));
            if t.age == 0 {
                let r = 2 * angle_diff(t.bearing, obs.radar_heading);
                turn_radar = Some(clamp_turn(r, MAX_RADAR_TURN));
                if obs.gun_heat.0 == 0 && -AIM_TOLERANCE <= d && d <= AIM_TOLERANCE {
                    if obs.energy.0 > 3000 {
                        fire = Some(Power(3000));
                    } else if obs.energy.0 > 100 {
                        fire = Some(Power(obs.energy.0 - 1));
                    }
                }
            }
        }
        None => {}
    }
    Action { fire, ahead: Some(100), turn_body, turn_gun, turn_radar }
}

// ---- candidate helpers below (untrusted) ----
fn clamp_turn(v: i32, m: i32) -> i32 {
    if v > m {
        m
    } else if v < -m {
        -m
    } else {
        v
    }
}
