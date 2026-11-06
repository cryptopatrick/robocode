//! The strategy layer: robots as `decide(&Observation) -> Action`.
//!
//! All quantities use the fixed-point units from [`crate::units`].

#[cfg(feature = "verus")]
use vstd::prelude::*;

pub mod targeting;

use crate::units::{Angle, Energy, GunHeat, Power};

/// What a robot knows about the enemy it last scanned.
#[cfg_attr(feature = "verus", verus_verify)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetInfo {
    /// Absolute bearing from this robot to the target.
    pub bearing: Angle,
    /// Distance in whole pixels.
    pub distance: i32,
    pub energy: Energy,
    pub heading: Angle,
    /// Velocity in decipixels per turn.
    pub velocity: i32,
    /// Turns since the target was last scanned (0 = scanned this turn).
    pub age: u32,
}

/// Everything a strategy sees at the start of a turn.
#[cfg_attr(feature = "verus", verus_verify)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observation {
    pub turn: u32,
    pub energy: Energy,
    pub gun_heat: GunHeat,
    pub heading: Angle,
    pub gun_heading: Angle,
    pub radar_heading: Angle,
    /// Velocity in decipixels per turn.
    pub velocity: i32,
    /// Position in whole pixels (y grows downward, as on screen).
    pub x: i32,
    pub y: i32,
    pub field_width: i32,
    pub field_height: i32,
    pub target: Option<TargetInfo>,
}

/// The commands a strategy issues for one turn. `None` leaves the pending
/// command unchanged. Turns are relative decidegrees; positive turns right.
#[cfg_attr(feature = "verus", verus_verify)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Action {
    pub fire: Option<Power>,
    /// Distance to move in whole pixels; negative moves backward.
    pub ahead: Option<i32>,
    pub turn_body: Option<i32>,
    pub turn_gun: Option<i32>,
    pub turn_radar: Option<i32>,
}

#[cfg(feature = "verus")]
verus! {
/// Observations the runner can produce: normalized angles and a battlefield
/// of at most 100000 px per side. Every strategy may assume this (it is the
/// runner's half of the contract; see docs/verification-receipt.md).
pub open spec fn well_formed(obs: Observation) -> bool {
    &&& 0 <= obs.heading.0 < 3600
    &&& 0 <= obs.gun_heading.0 < 3600
    &&& 0 <= obs.radar_heading.0 < 3600
    &&& 0 <= obs.field_width <= 100_000
    &&& 0 <= obs.field_height <= 100_000
    &&& obs.target matches Some(t) ==> 0 <= t.bearing.0 < 3600
}
}

/// Executable check of [`well_formed`]: the runner asserts it on every
/// observation it builds.
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    ensures r == well_formed(*obs),
))]
pub fn is_well_formed(obs: &Observation) -> bool {
    let angles_ok = 0 <= obs.heading.0 && obs.heading.0 < 3600
        && 0 <= obs.gun_heading.0 && obs.gun_heading.0 < 3600
        && 0 <= obs.radar_heading.0 && obs.radar_heading.0 < 3600;
    let field_ok = 0 <= obs.field_width && obs.field_width <= 100_000
        && 0 <= obs.field_height && obs.field_height <= 100_000;
    let target_ok = match obs.target {
        Some(t) => 0 <= t.bearing.0 && t.bearing.0 < 3600,
        None => true,
    };
    angles_ok && field_ok && target_ok
}

/// A robot's brain.
pub trait Strategy {
    fn name(&self) -> &str;
    fn decide(&mut self, obs: &Observation) -> Action;
}
