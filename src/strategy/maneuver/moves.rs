//! The five maneuvers. Each is a pure function of the observation, the
//! memory and the risk parameters. Every one turns the radar, and every shot
//! goes through `fire_filter`.
//!
//! Body turns are relative decidegrees and are not clamped here; the engine
//! spreads a large turn over several turns. Gun and radar turns are clamped
//! to the engine's per-turn limits.

use super::memory::ManeuverMemory;
use super::{
    bearing_to, fire_filter, gun_aim, radar_lock, Maneuver,
    RiskParams,
};
use crate::strategy::{Action, Observation};
use crate::units::{angle_diff, Angle, Power};

/// Within this many pixels of a waypoint, the next one is chosen.
const WAYPOINT_REACHED: i32 = 20;
/// Within this many pixels of the centre, Search loops instead of aiming.
const CENTRE_ZONE: i32 = 50;

impl Maneuver {
    pub fn run(
        self,
        obs: &Observation,
        mem: &mut ManeuverMemory,
        p: &RiskParams,
    ) -> Action {
        match self {
            Maneuver::Search => search(obs, mem, p),
            Maneuver::Patrol => patrol(obs, mem, p),
            Maneuver::Defensive => defensive(obs, mem, p),
            Maneuver::Evade => evade(obs, mem, p),
            Maneuver::Attack => attack(obs, mem, p),
        }
    }
}

fn distance(obs: &Observation, x: i32, y: i32) -> i32 {
    let dx = (x - obs.x) as f64;
    let dy = (y - obs.y) as f64;
    dx.hypot(dy).round() as i32
}

/// Gun turn that follows the radar.
fn gun_follows_radar(obs: &Observation) -> i32 {
    angle_diff(obs.radar_heading, obs.gun_heading).clamp(-200, 200)
}

/// Radar lock, nudged by one decidegree when the radar is exactly on the
/// target so the radar command is never zero.
fn lock(obs: &Observation) -> i32 {
    match radar_lock(obs) {
        0 => 1,
        r => r,
    }
}

/// Turn toward a point and advance up to 100 px.
fn drive_to(obs: &Observation, x: i32, y: i32) -> (i32, i32) {
    let turn = angle_diff(bearing_to(obs, x, y), obs.heading);
    (turn, distance(obs, x, y).min(100))
}

pub fn search(
    obs: &Observation,
    _mem: &mut ManeuverMemory,
    _p: &RiskParams,
) -> Action {
    let (cx, cy) = (obs.field_width / 2, obs.field_height / 2);
    let (turn_body, ahead) = if distance(obs, cx, cy) <= CENTRE_ZONE {
        (50, 100)
    } else {
        drive_to(obs, cx, cy)
    };
    Action {
        fire: None,
        ahead: Some(ahead),
        turn_body: Some(turn_body),
        turn_gun: Some(gun_follows_radar(obs)),
        turn_radar: Some(450),
    }
}

/// The patrol route: the field's centre roads, via the centre each time.
fn waypoint(obs: &Observation, p: &RiskParams, index: u8) -> (i32, i32) {
    let (w, h) = (obs.field_width, obs.field_height);
    let (cx, cy) = (w / 2, h / 2);
    let k = p.keep_clear + 40;
    match index % 8 {
        0 => (cx, k),
        1 => (cx, cy),
        2 => (w - k, cy),
        3 => (cx, cy),
        4 => (cx, h - k),
        5 => (cx, cy),
        6 => (k, cy),
        _ => (cx, cy),
    }
}

pub fn patrol(
    obs: &Observation,
    mem: &mut ManeuverMemory,
    p: &RiskParams,
) -> Action {
    let (mut x, mut y) = waypoint(obs, p, mem.waypoint);
    if distance(obs, x, y) <= WAYPOINT_REACHED {
        mem.waypoint = (mem.waypoint + 1) % 8;
        (x, y) = waypoint(obs, p, mem.waypoint);
    }
    let (turn_body, ahead) = drive_to(obs, x, y);
    Action {
        fire: None,
        ahead: Some(ahead),
        turn_body: Some(turn_body),
        turn_gun: Some(gun_follows_radar(obs)),
        turn_radar: Some(450),
    }
}

pub fn defensive(
    obs: &Observation,
    _mem: &mut ManeuverMemory,
    p: &RiskParams,
) -> Action {
    let Some(t) = obs.target else {
        return Action {
            turn_radar: Some(lock(obs)),
            ..Action::default()
        };
    };
    // Side-on: whichever of bearing +/- 900 needs the smaller body turn.
    let left = angle_diff(Angle((t.bearing.0 - 900).rem_euclid(3600)), obs.heading);
    let right = angle_diff(Angle((t.bearing.0 + 900).rem_euclid(3600)), obs.heading);
    let turn_body = if right.abs() <= left.abs() { right } else { left };
    let aim = gun_aim(obs);
    let fire = if aim.abs() <= p.aim_tolerance {
        fire_filter(obs, Power(p.shot_power.0.min(1000)))
    } else {
        None
    };
    Action {
        fire,
        ahead: Some(if t.distance < p.engage_distance { -50 } else { 0 }),
        turn_body: Some(turn_body),
        turn_gun: Some(aim),
        turn_radar: Some(lock(obs)),
    }
}

/// Evade reads `turns_in_maneuver == 0` to detect entry; the caller owns
/// that counter.
pub fn evade(
    obs: &Observation,
    mem: &mut ManeuverMemory,
    _p: &RiskParams,
) -> Action {
    if mem.turns_in_maneuver == 0 {
        mem.evade_dir = if mem.next_bit() { 1 } else { -1 };
    }
    let dir = mem.evade_dir as i32;
    let turn_body = obs.target.map(|t| {
        let heading =
            Angle((t.bearing.0 + 900 * dir).rem_euclid(3600));
        angle_diff(heading, obs.heading)
    });
    Action {
        fire: None,
        ahead: Some(100 * dir.signum()),
        turn_body,
        turn_gun: Some(gun_aim(obs)),
        turn_radar: Some(lock(obs)),
    }
}

pub fn attack(
    obs: &Observation,
    _mem: &mut ManeuverMemory,
    p: &RiskParams,
) -> Action {
    let Some(t) = obs.target else {
        return Action {
            turn_radar: Some(lock(obs)),
            ..Action::default()
        };
    };
    let aim = gun_aim(obs);
    let fire = if aim.abs() <= p.aim_tolerance {
        fire_filter(obs, p.shot_power)
    } else {
        None
    };
    Action {
        fire,
        ahead: Some((t.distance - p.engage_distance).clamp(-100, 100)),
        turn_body: Some(angle_diff(t.bearing, obs.heading)),
        turn_gun: Some(aim),
        turn_radar: Some(lock(obs)),
    }
}
