//! TRUSTED TEMPLATE (robocheck). The candidate supplies only the body of
//! `decide` and helper functions below the marked line; the reference spec on
//! `decide` is human-written and may not be changed by the candidate.
//! Reference: SafeShooter's properties, src/robots/safe_shooter.rs.
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

use robocode::robots::safe_shooter::{AIM_TOLERANCE, MAX_GUN_TURN, MAX_RADAR_TURN, WALL_MARGIN};
use robocode::strategy::{Action, Observation};
use robocode::units::{Angle, Power, angle_diff};
#[cfg(verus_only)]
use robocode::robots::safe_shooter::{aimed, heading_into_near_wall, ready, reverse_heading};
#[cfg(verus_only)]
use robocode::strategy::well_formed;
#[cfg(verus_only)]
use robocode::units::{MAX_POWER, MIN_POWER, spec_angle_diff};

/// The candidate's decision for one turn, checked against the reference spec.
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires well_formed(*obs),
    ensures
        r.fire matches Some(p) ==> MIN_POWER.0 <= p.0 <= MAX_POWER.0, // SAFE-001
        r.fire matches Some(p) ==> obs.energy.0 > p.0, // SAFE-002
        r.fire is Some ==> obs.gun_heat.0 == 0, // SAFE-003
        r.fire is Some ==> (obs.target matches Some(t) && t.age == 0), // SAFE-004
        r.fire is Some ==> aimed(*obs), // SAFE-005
        r.ahead matches Some(a) ==> (a > 0 ==> !heading_into_near_wall(*obs, obs.heading.0 as int)), // SAFE-006
        r.ahead matches Some(a) ==> (a < 0 ==> !heading_into_near_wall(*obs, reverse_heading(obs.heading.0 as int))), // SAFE-006
        r.fire is Some <==> ready(*obs), // SAFE-007
))]
pub fn decide(obs: &Observation) -> Action {
{{BODY}}
}

/// The candidate as a robot for the battle runner.
pub struct Candidate;

impl robocode::strategy::Strategy for Candidate {
    fn name(&self) -> &str {
        "Candidate"
    }
    fn decide(&mut self, obs: &Observation) -> Action {
        decide(obs)
    }
}

// ---- candidate helpers below (untrusted) ----
{{HELPERS}}
