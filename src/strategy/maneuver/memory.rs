//! What a maneuvering robot remembers between turns: a seeded RNG, the
//! target's last fresh state, and shot detection.

use super::target_pos;
use crate::rules::ROBOT_RADIUS;
use crate::strategy::Observation;
use crate::units::Energy;

const DEFAULT_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
/// Wall distance in pixels within which a stop counts as a wall hit.
const WALL_ZONE: i32 = ROBOT_RADIUS as i32 + 2;
/// Distance in pixels within which a drop counts as a collision.
const COLLISION_DISTANCE: i32 = 40;

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
