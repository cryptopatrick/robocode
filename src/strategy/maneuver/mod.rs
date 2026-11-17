//! The maneuver library: what a robot is doing (`Maneuver`) and how boldly
//! (`Risk`). Task 9 adds the maneuvers themselves; this is their base.

pub mod memory;
pub mod moves;

use crate::strategy::Observation;
use crate::units::{angle_diff, Angle, Energy, Power};

/// What the robot is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Maneuver {
    Search,
    Patrol,
    Defensive,
    Evade,
    Attack,
}

impl Maneuver {
    pub fn name(self) -> &'static str {
        match self {
            Maneuver::Search => "search",
            Maneuver::Patrol => "patrol",
            Maneuver::Defensive => "defensive",
            Maneuver::Evade => "evade",
            Maneuver::Attack => "attack",
        }
    }
}

/// How boldly the robot does it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Risk {
    Cautious,
    Normal,
    Aggressive,
}

impl Risk {
    pub fn name(self) -> &'static str {
        match self {
            Risk::Cautious => "cautious",
            Risk::Normal => "normal",
            Risk::Aggressive => "aggressive",
        }
    }

    /// The starting parameter values from the spec table.
    pub fn params(self) -> RiskParams {
        match self {
            Risk::Cautious => RiskParams {
                reserve: Energy(40000),
                engage_distance: 450,
                aim_tolerance: 20,
                shot_power: Power(1000),
                keep_clear: 80,
                evade_turns: 20,
                stale_after: 10,
            },
            Risk::Normal => RiskParams {
                reserve: Energy(20000),
                engage_distance: 300,
                aim_tolerance: 40,
                shot_power: Power(2000),
                keep_clear: 50,
                evade_turns: 12,
                stale_after: 20,
            },
            Risk::Aggressive => RiskParams {
                reserve: Energy(5000),
                engage_distance: 150,
                aim_tolerance: 80,
                shot_power: Power(3000),
                keep_clear: 25,
                evade_turns: 6,
                stale_after: 30,
            },
        }
    }
}

/// A maneuver and its risk level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intent {
    pub maneuver: Maneuver,
    pub risk: Risk,
}

/// The numbers a risk level stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskParams {
    /// Energy below which the robot stops being bold.
    pub reserve: Energy,
    /// Distance in pixels at which Attack stops closing in.
    pub engage_distance: i32,
    /// Gun misalignment in decidegrees that still counts as aimed.
    pub aim_tolerance: i32,
    pub shot_power: Power,
    /// Pixels to keep between the robot and a wall.
    pub keep_clear: i32,
    /// Turns Evade lasts after a detected shot.
    pub evade_turns: u32,
    /// Turns without a scan after which the target counts as lost.
    pub stale_after: u32,
}

/// The only way maneuvers fire: a fresh target (`age == 0`) and a power
/// strictly below the robot's own energy (firing everything would kill it).
pub fn fire_filter(obs: &Observation, power: Power) -> Option<Power> {
    match obs.target {
        Some(t) if t.age == 0 && power.0 < obs.energy.0 => Some(power),
        _ => None,
    }
}

/// The target's position in whole pixels, from the last scan.
pub fn target_pos(obs: &Observation) -> Option<(i32, i32)> {
    let t = obs.target?;
    let rad = (t.bearing.0 as f64) / 10.0 * std::f64::consts::PI / 180.0;
    let d = t.distance as f64;
    Some((
        obs.x + (d * rad.sin()).round() as i32,
        obs.y - (d * rad.cos()).round() as i32,
    ))
}

/// Absolute bearing in decidegrees (0 = up, clockwise) from the robot to a point.
pub fn bearing_to(obs: &Observation, x: i32, y: i32) -> Angle {
    let dx = (x - obs.x) as f64;
    let dy = (obs.y - y) as f64;
    let deg = dx.atan2(dy).to_degrees();
    let deci = (deg * 10.0).round() as i32;
    Angle(deci.rem_euclid(3600))
}

/// Radar turn that keeps the radar on the target (twice the error, so it
/// overshoots and keeps the target in the beam), within the 45 degree limit.
pub fn radar_lock(obs: &Observation) -> i32 {
    match obs.target {
        Some(t) => {
            (2 * angle_diff(t.bearing, obs.radar_heading)).clamp(-450, 450)
        }
        None => 450,
    }
}

/// Gun turn toward the target, within the 20 degree limit.
pub fn gun_aim(obs: &Observation) -> i32 {
    match obs.target {
        Some(t) => angle_diff(t.bearing, obs.gun_heading).clamp(-200, 200),
        None => 0,
    }
}
