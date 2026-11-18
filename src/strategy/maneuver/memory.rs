//! What a maneuvering robot remembers between turns: a seeded RNG, the
//! target's last fresh state, and shot detection.

use super::target_pos;
use crate::rules::ROBOT_RADIUS;
use crate::strategy::{Action, Observation};
use crate::units::Energy;

const DEFAULT_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
/// Wall distance in pixels within which a stop counts as a wall hit.
const WALL_ZONE: i32 = ROBOT_RADIUS as i32 + 2;
/// Distance in pixels within which a drop counts as a collision.
const COLLISION_DISTANCE: i32 = 40;
/// Consecutive turns at velocity 0 after a move command that count as stuck.
pub const STUCK_TURNS: u8 = 3;
/// Turns a recovery lasts.
pub const RECOVERY_TURNS: u8 = 10;
/// Distance in pixels of the one back-off command.
pub const RECOVERY_BACKOFF: i32 = 60;
/// Body turn in decidegrees (90 degrees) per recovery turn.
pub const RECOVERY_TURN: i32 = 900;

/// Pixels kept between the robot's centre and a wall. The robot's radius is
/// 18, and the engine can carry a robot a few pixels past a commanded
/// distance while it brakes, so a smaller margin still ends in the wall.
pub const WALL_CLEARANCE: i32 = 30;
/// Least room in pixels for a recovery back-off to be worth issuing.
const MIN_BACKOFF: i32 = 10;

/// How far the robot can drive in direction `dir` (1 forward, -1 backward)
/// while turning by `turn` decidegrees before it comes within
/// [`WALL_CLEARANCE`] of a wall (negative when already inside). The body
/// swings while driving, so the least room over the start, middle and end
/// headings is used.
pub fn wall_room(obs: &Observation, dir: i32, turn: i32) -> i32 {
    let room = |pos: i32, size: i32, d: f64| -> f64 {
        if d > 1e-9 {
            ((size - WALL_CLEARANCE - pos) as f64) / d
        } else if d < -1e-9 {
            ((pos - WALL_CLEARANCE) as f64) / -d
        } else {
            f64::INFINITY
        }
    };
    let mut best = i32::MAX;
    for k in 0..=2 {
        let h = obs.heading.0 + turn * k / 2;
        let rad = (h as f64) / 10.0 * std::f64::consts::PI / 180.0;
        let (dx, dy) = (rad.sin() * dir as f64, -rad.cos() * dir as f64);
        let t = room(obs.x, obs.field_width, dx).min(room(
            obs.y,
            obs.field_height,
            dy,
        ));
        best = best.min(t.floor().clamp(-100_000.0, 100_000.0) as i32);
    }
    best
}

/// Clips a commanded `ahead` to the room left before a wall along the
/// current heading. A command into a wall becomes `Some(0)`.
pub fn wall_guard(obs: &Observation, mut action: Action) -> Action {
    if let Some(a) = action.ahead.filter(|a| *a != 0) {
        let room = wall_room(obs, a.signum(), 0).max(0);
        action.ahead = Some(a.signum() * a.abs().min(room));
    }
    action
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManeuverMemory {
    pub waypoint: u8,
    pub last_target_energy: Option<Energy>,
    pub own_energy_at_last_scan: Energy,
    pub last_target_pos: Option<(i32, i32)>,
    pub last_target_velocity: i32,
    pub evade_dir: i8,
    pub turns_in_maneuver: u32,
    pub rng: u64,
    /// Sign of the previous Action's `ahead` (0 = none or zero).
    pub last_ahead: i8,
    /// Consecutive turns at velocity 0 after a move command.
    pub stuck_turns: u8,
    /// Recovery turns still to run.
    pub recovery_left: u8,
}

impl ManeuverMemory {
    pub fn new(seed: u64) -> Self {
        ManeuverMemory {
            waypoint: 0,
            last_target_energy: None,
            own_energy_at_last_scan: Energy(0),
            last_target_pos: None,
            last_target_velocity: 0,
            evade_dir: 1,
            turns_in_maneuver: 0,
            rng: if seed == 0 { DEFAULT_SEED } else { seed },
            last_ahead: 0,
            stuck_turns: 0,
            recovery_left: 0,
        }
    }

    /// xorshift64; returns the low bit.
    pub fn next_bit(&mut self) -> bool {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        x & 1 == 1
    }

    /// Stuck recovery, built only from the observation's velocity.
    ///
    /// When the previous move command was followed by 3 turns at velocity 0,
    /// the first recovery turn issues one back-off against the direction it
    /// was stuck in plus a 90 degree turn. For the rest of the recovery the
    /// movement is left to the engine, so the pending commands finish. If
    /// the robot is still at velocity 0 on a later recovery turn, recovery
    /// ends and stuck detection starts over. Radar, gun and fire are left as
    /// the maneuver chose them.
    pub fn recover(
        &mut self,
        obs: &Observation,
        mut action: Action,
    ) -> Action {
        if self.recovery_left > 0 {
            if obs.velocity == 0 {
                self.recovery_left = 0; // still stuck: abort
                self.stuck_turns = 0;
            } else {
                self.recovery_left -= 1;
                action.ahead = None;
                action.turn_body = None;
                return action;
            }
        }
        if self.last_ahead != 0 && obs.velocity == 0 {
            self.stuck_turns = self.stuck_turns.saturating_add(1);
        } else {
            self.stuck_turns = 0;
        }
        if self.stuck_turns >= STUCK_TURNS {
            self.stuck_turns = 0;
            self.recovery_left = RECOVERY_TURNS - 1;
            let sign = if self.next_bit() { 1 } else { -1 };
            let turn = sign * RECOVERY_TURN;
            // Back off against the direction it was stuck in; if a wall is
            // too close that way, the other way; if both, only turn.
            let mut back = -(self.last_ahead as i32);
            if wall_room(obs, back, turn) < MIN_BACKOFF {
                back = -back;
            }
            let room = wall_room(obs, back, turn);
            if room >= MIN_BACKOFF {
                action.ahead = Some(back * room.min(RECOVERY_BACKOFF));
                self.last_ahead = back as i8;
            } else {
                action.ahead = None;
                self.last_ahead = 0;
            }
            action.turn_body = Some(turn);
        } else {
            self.last_ahead = action.ahead.map_or(0, |a| a.signum() as i8);
        }
        action
    }

    /// Forget stuck state (for maneuvers that deliberately stand still, and
    /// on entering Evade).
    pub fn clear_stuck(&mut self) {
        self.last_ahead = 0;
        self.stuck_turns = 0;
        self.recovery_left = 0;
    }

    /// Call once per turn before choosing. Returns true when a shot by the
    /// target is detected, and updates the memory.
    ///
    /// A shot is only detected between two consecutive fresh scans; a gap
    /// resets it, so a drop across the gap is not taken for a shot.
    pub fn observe(&mut self, obs: &Observation) -> bool {
        let Some(t) = obs.target.filter(|t| t.age == 0) else {
            // A stale scan breaks the chain: detection needs two fresh scans.
            self.last_target_energy = None;
            return false;
        };
        let pos = target_pos(obs);
        let mut shot = false;
        if let Some(prev) = self.last_target_energy {
            let drop = prev.0.saturating_sub(t.energy.0);
            let in_range = (100..=3000).contains(&drop) && prev.0 > t.energy.0;
            let own_hit = obs.energy > self.own_energy_at_last_scan;
            let wall = self.last_target_velocity != 0
                && t.velocity == 0
                && pos.is_some_and(|(px, py)| {
                    px <= WALL_ZONE
                        || py <= WALL_ZONE
                        || px >= obs.field_width - WALL_ZONE
                        || py >= obs.field_height - WALL_ZONE
                });
            let collision = t.distance <= COLLISION_DISTANCE;
            shot = in_range && !own_hit && !wall && !collision;
        }
        self.last_target_energy = Some(t.energy);
        self.own_energy_at_last_scan = obs.energy;
        self.last_target_pos = pos;
        self.last_target_velocity = t.velocity;
        shot
    }
}
