//! `Circler`: a sample robot written only against [`crate::api`].
//!
//! It drives in wide circles, reverses when it hits a wall, locks its radar
//! and gun onto the robot it scans, fires when the gun is lined up, and turns
//! side-on to incoming fire. Copy it as a starting point for your own robot.

use crate::api::{
    Commands, HitByBulletEvent, HitWallEvent, Robot, ScannedRobotEvent, Status, relative_bearing,
};

/// Degrees within which the gun counts as lined up.
const AIM_TOLERANCE: f64 = 5.0;

pub struct Circler {
    /// 1.0 = driving forward, -1.0 = driving backward.
    direction: f64,
    /// Whether the radar found a robot this turn (then `run` keeps it locked).
    locked: bool,
    /// A body turn chosen by an event handler this turn, if any.
    turn: Option<f64>,
}

impl Default for Circler {
    fn default() -> Self {
        Circler { direction: 1.0, locked: false, turn: None }
    }
}

impl Robot for Circler {
    fn name(&self) -> &str {
        "Circler"
    }

    fn run(&mut self, _status: &Status, cmd: &mut Commands) {
        cmd.set_ahead(100.0 * self.direction);
        cmd.set_turn_right(self.turn.take().unwrap_or(15.0));
        if !self.locked {
            cmd.set_turn_radar_right(45.0);
        }
        self.locked = false;
    }

    fn on_scanned_robot(&mut self, e: &ScannedRobotEvent, s: &Status, cmd: &mut Commands) {
        if self.locked {
            return; // already locked onto the nearest robot this turn
        }
        self.locked = true;
        // Sweep the radar past the target by as much again, so it stays in view.
        cmd.set_turn_radar_right(2.0 * relative_bearing(e.absolute_bearing, s.radar_heading));
        let gun_error = relative_bearing(e.absolute_bearing, s.gun_heading);
        cmd.set_turn_gun_right(gun_error);
        if gun_error.abs() <= AIM_TOLERANCE {
            cmd.set_fire(2.0); // ignored by the engine while the gun is hot
        }
    }

    fn on_hit_wall(&mut self, _e: &HitWallEvent, _s: &Status, _cmd: &mut Commands) {
        self.direction = -self.direction;
    }

    fn on_hit_by_bullet(&mut self, e: &HitByBulletEvent, _s: &Status, _cmd: &mut Commands) {
        // Turn side-on to the shooter.
        self.turn = Some(relative_bearing(e.bearing + 90.0, 0.0));
    }
}
