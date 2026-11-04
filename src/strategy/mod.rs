//! The strategy layer: robots as `decide(&Observation) -> Action`.
//!
//! All quantities use the fixed-point units from [`crate::units`].

#[cfg(feature = "verus")]
use vstd::prelude::*;

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

/// A robot's brain.
pub trait Strategy {
    fn name(&self) -> &str;
    fn decide(&mut self, obs: &Observation) -> Action;
}
