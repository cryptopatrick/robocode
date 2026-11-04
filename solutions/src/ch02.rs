//! Chapter 2: `requires` and `ensures`, with SafeShooter.
use robocode::robots::safe_shooter::{decide, is_aimed};
// Spec functions exist only while Verus runs.
#[cfg(verus_only)]
use robocode::robots::safe_shooter::{safe_to_fire, well_formed};
use robocode::strategy::Observation;
use robocode::units::Power;
use vstd::prelude::*;

verus! {

// Exercise 2.1: a careful trigger. Write the `requires` and `ensures`.
pub fn careful_fire(obs: &Observation) -> (r: Option<Power>)
    requires
        well_formed(*obs),
    ensures
        r matches Some(p) ==> safe_to_fire(*obs, p),
{
    let ready = match obs.target {
        Some(t) => t.age == 0,
        None => false,
    };
    if ready && obs.gun_heat.0 == 0 && obs.energy.0 >= 1000 && is_aimed(obs) {
        Some(Power(1000))
    } else {
        None
    }
}

pub fn check_careful_fire(obs: &Observation)
    requires
        well_formed(*obs),
{
    if let Some(p) = careful_fire(obs) {
        assert(obs.gun_heat.0 == 0);
        assert(obs.energy.0 >= p.0);
    }
}

// Exercise 2.2: using SafeShooter's proven `decide`. Add the missing `requires`.
pub fn check_safe_shooter(obs: &Observation)
    requires
        well_formed(*obs),
{
    let a = decide(obs);
    if let Some(p) = a.fire {
        assert(obs.gun_heat.0 == 0);
        assert(obs.energy.0 >= p.0);
        assert(obs.target is Some);
    }
}

} // verus!
