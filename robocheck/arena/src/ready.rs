//! An executable twin of SafeShooter's `ready` spec, proved equal to it.
#![allow(clippy::manual_range_contains)]

#[cfg(feature = "verus")]
use vstd::prelude::*;

use robocode::robots::safe_shooter::is_aimed;
use robocode::strategy::Observation;
#[cfg(verus_only)]
use robocode::robots::safe_shooter::{aimed, ready};
#[cfg(verus_only)]
use robocode::strategy::well_formed;

/// Whether SafeShooter would call this observation ready to fire: a target
/// scanned this turn, a cool gun aimed at it, and more than 1.0 energy.
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires well_formed(*obs),
    ensures r == ready(*obs),
))]
pub fn is_ready(obs: &Observation) -> bool {
    let scanned_now = match obs.target {
        Some(t) => t.age == 0,
        None => false,
    };
    scanned_now && obs.gun_heat.0 == 0 && is_aimed(obs) && obs.energy.0 > 1000
}
