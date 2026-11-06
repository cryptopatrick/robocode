//! The Robot API: write your own robot.
//!
//! Modelled on classic RoboCode's `AdvancedRobot`. Each turn your robot's
//! event handlers are called for what happened last turn, then [`Robot::run`].
//! Every call gets the robot's [`Status`] and a [`Commands`] buffer; the
//! commands you set are carried out when the turn executes. Units are pixels
//! and degrees: 0° points up and angles grow clockwise.
//!
//! ```
//! use robocode::api::{Commands, Robot, ScannedRobotEvent, Status};
//!
//! struct Spinner;
//!
//! impl Robot for Spinner {
//!     fn name(&self) -> &str { "MySpinner" }
//!     fn run(&mut self, _status: &Status, cmd: &mut Commands) {
//!         cmd.set_ahead(100.0);
//!         cmd.set_turn_right(10.0);
//!         cmd.set_turn_radar_right(45.0);
//!     }
//!     fn on_scanned_robot(&mut self, _e: &ScannedRobotEvent, s: &Status, cmd: &mut Commands) {
//!         if s.gun_heat == 0.0 { cmd.set_fire(1.0); }
//!     }
//! }
//! ```
//!
//! See `docs/robot-api.md` for the full guide.

use crate::physics::RobotState;
pub use crate::physics::TurretMode;
use crate::strategy::{Action, Observation};

/// Your robot at the start of the turn.
#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    /// Turn number (0 before the first turn has run).
    pub time: u64,
    /// Your robot's name in this battle (numbered if several share it).
    pub name: String,
    /// Opponents still alive.
    pub others: usize,
    pub x: f64,
    pub y: f64,
    pub energy: f64,
    /// Pixels per turn; negative when moving backward.
    pub velocity: f64,
    /// The gun can fire only at 0.
    pub gun_heat: f64,
    pub heading: f64,
    pub gun_heading: f64,
    pub radar_heading: f64,
    /// Pixels still to travel from earlier `set_ahead` / `set_back` calls.
    pub distance_remaining: f64,
    /// Degrees still to turn (body, gun, radar).
    pub turn_remaining: f64,
    pub gun_turn_remaining: f64,
    pub radar_turn_remaining: f64,
    pub battlefield_width: f64,
    pub battlefield_height: f64,
    /// Gun heat removed per turn.
    pub gun_cooling_rate: f64,
}

/// Your radar swept over another robot.
#[derive(Debug, Clone, PartialEq)]
pub struct ScannedRobotEvent {
    pub name: String,
    /// Bearing relative to your heading, in `(-180, 180]`.
    pub bearing: f64,
    /// Absolute bearing, in `[0, 360)`.
    pub absolute_bearing: f64,
    pub distance: f64,
    pub energy: f64,
    pub heading: f64,
    pub velocity: f64,
}

/// A bullet hit you.
#[derive(Debug, Clone, PartialEq)]
pub struct HitByBulletEvent {
    /// Who fired it.
    pub name: String,
    /// Bearing to the shooter relative to your heading, in `(-180, 180]`.
    pub bearing: f64,
    pub power: f64,
}

/// One of your bullets hit another robot.
#[derive(Debug, Clone, PartialEq)]
pub struct BulletHitEvent {
    /// The robot you hit.
    pub name: String,
    /// Its energy after the hit.
    pub energy: f64,
    pub power: f64,
}

/// One of your bullets collided with another bullet; both are gone.
#[derive(Debug, Clone, PartialEq)]
pub struct BulletHitBulletEvent {
    /// Owner of the other bullet.
    pub name: String,
    /// Your bullet's power.
    pub power: f64,
    pub other_power: f64,
}

/// One of your bullets left the battlefield without hitting anything.
#[derive(Debug, Clone, PartialEq)]
pub struct BulletMissedEvent {
    pub power: f64,
}

/// You drove into a wall.
#[derive(Debug, Clone, PartialEq)]
pub struct HitWallEvent {
    /// Bearing to the wall relative to your heading, in `(-180, 180]`.
    pub bearing: f64,
}

/// You collided with another robot.
#[derive(Debug, Clone, PartialEq)]
pub struct HitRobotEvent {
    pub name: String,
    /// Bearing to the other robot relative to your heading, in `(-180, 180]`.
    pub bearing: f64,
    /// The other robot's energy after the collision.
    pub energy: f64,
    /// True if you drove into it.
    pub at_fault: bool,
}

/// Another robot died.
#[derive(Debug, Clone, PartialEq)]
pub struct RobotDeathEvent {
    pub name: String,
}

/// A round finished (sent to every robot, alive or not).
#[derive(Debug, Clone, PartialEq)]
pub struct RoundEndedEvent {
    /// 0-based round number.
    pub round: u32,
    pub total_rounds: u32,
    /// Turns the round lasted.
    pub turns: u64,
}

/// Something that happened last turn. Delivered in this order.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    ScannedRobot(ScannedRobotEvent),
    HitByBullet(HitByBulletEvent),
    BulletHit(BulletHitEvent),
    BulletHitBullet(BulletHitBulletEvent),
    BulletMissed(BulletMissedEvent),
    HitWall(HitWallEvent),
    HitRobot(HitRobotEvent),
    RobotDeath(RobotDeathEvent),
}

/// What to do this turn. A later call of the same kind replaces an earlier
/// one; kinds you never set keep whatever is still pending from before.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Commands {
    ahead: Option<f64>,
    turn_body: Option<f64>,
    turn_gun: Option<f64>,
    turn_radar: Option<f64>,
    fire: Option<f64>,
    max_velocity: Option<f64>,
    max_turn_rate: Option<f64>,
    turret_mode: Option<TurretMode>,
    adjust_gun_for_robot_turn: Option<bool>,
    adjust_radar_for_robot_turn: Option<bool>,
    adjust_radar_for_gun_turn: Option<bool>,
}

impl Commands {
    /// Move forward `px` pixels (negative moves backward).
    pub fn set_ahead(&mut self, px: f64) {
        self.ahead = Some(px);
    }
    /// Move backward `px` pixels.
    pub fn set_back(&mut self, px: f64) {
        self.set_ahead(-px);
    }
    /// Turn the body clockwise by `deg` degrees.
    pub fn set_turn_right(&mut self, deg: f64) {
        self.turn_body = Some(deg);
    }
    pub fn set_turn_left(&mut self, deg: f64) {
        self.set_turn_right(-deg);
    }
    pub fn set_turn_gun_right(&mut self, deg: f64) {
        self.turn_gun = Some(deg);
    }
    pub fn set_turn_gun_left(&mut self, deg: f64) {
        self.set_turn_gun_right(-deg);
    }
    pub fn set_turn_radar_right(&mut self, deg: f64) {
        self.turn_radar = Some(deg);
    }
    pub fn set_turn_radar_left(&mut self, deg: f64) {
        self.set_turn_radar_right(-deg);
    }
    /// Fire a bullet of the given power (0.1 to 3.0) if the gun is cool.
    pub fn set_fire(&mut self, power: f64) {
        self.fire = Some(power);
    }

    /// Never go faster than `velocity` px/turn (0 to 8) until changed.
    pub fn set_max_velocity(&mut self, velocity: f64) {
        self.max_velocity = Some(velocity);
    }
    /// Never turn the body faster than `degrees` per turn (0 to 10) until changed.
    pub fn set_max_turn_rate(&mut self, degrees: f64) {
        self.max_turn_rate = Some(degrees);
    }

    /// Java-style turret: the body drags the gun and radar (see [`TurretMode`]).
    /// Stays in effect until changed. Ported Java robots want `TurretMode::Java`.
    pub fn set_turret_mode(&mut self, mode: TurretMode) {
        self.turret_mode = Some(mode);
    }
    /// Java `setAdjustGunForRobotTurn` (only matters in `TurretMode::Java`).
    pub fn set_adjust_gun_for_robot_turn(&mut self, adjust: bool) {
        self.adjust_gun_for_robot_turn = Some(adjust);
    }
    /// Java `setAdjustRadarForRobotTurn` (only matters in `TurretMode::Java`).
    pub fn set_adjust_radar_for_robot_turn(&mut self, adjust: bool) {
        self.adjust_radar_for_robot_turn = Some(adjust);
    }
    /// Java `setAdjustRadarForGunTurn` (only matters in `TurretMode::Java`).
    pub fn set_adjust_radar_for_gun_turn(&mut self, adjust: bool) {
        self.adjust_radar_for_gun_turn = Some(adjust);
    }

    pub fn turret_mode(&self) -> Option<TurretMode> {
        self.turret_mode
    }
    pub fn adjust_gun_for_robot_turn(&self) -> Option<bool> {
        self.adjust_gun_for_robot_turn
    }
    pub fn adjust_radar_for_robot_turn(&self) -> Option<bool> {
        self.adjust_radar_for_robot_turn
    }
    pub fn adjust_radar_for_gun_turn(&self) -> Option<bool> {
        self.adjust_radar_for_gun_turn
    }

    pub fn max_velocity(&self) -> Option<f64> {
        self.max_velocity
    }
    pub fn max_turn_rate(&self) -> Option<f64> {
        self.max_turn_rate
    }

    pub fn ahead(&self) -> Option<f64> {
        self.ahead
    }
    pub fn turn_body(&self) -> Option<f64> {
        self.turn_body
    }
    pub fn turn_gun(&self) -> Option<f64> {
        self.turn_gun
    }
    pub fn turn_radar(&self) -> Option<f64> {
        self.turn_radar
    }
    pub fn fire(&self) -> Option<f64> {
        self.fire
    }

    /// Hands the commands to the engine.
    pub(crate) fn apply_to(&self, robot: &mut RobotState) {
        if let Some(mode) = self.turret_mode {
            robot.turret.mode = mode;
        }
        if let Some(a) = self.adjust_gun_for_robot_turn {
            robot.set_adjust_gun_for_robot_turn(a);
        }
        if let Some(a) = self.adjust_radar_for_robot_turn {
            robot.set_adjust_radar_for_robot_turn(a);
        }
        if let Some(a) = self.adjust_radar_for_gun_turn {
            robot.set_adjust_radar_for_gun_turn(a);
        }
        if let Some(v) = self.max_velocity {
            robot.set_max_velocity(v);
        }
        if let Some(t) = self.max_turn_rate {
            robot.set_max_turn_rate(t);
        }
        if let Some(p) = self.fire {
            robot.set_fire(p);
        }
        if let Some(d) = self.ahead {
            robot.set_ahead(d);
        }
        if let Some(t) = self.turn_body {
            robot.set_turn_right(t);
        }
        if let Some(t) = self.turn_gun {
            robot.set_turn_gun_right(t);
        }
        if let Some(t) = self.turn_radar {
            robot.set_turn_radar_right(t);
        }
    }
}

/// A robot. Implement [`Robot::name`] and [`Robot::run`]; override the
/// event handlers you care about.
pub trait Robot {
    fn name(&self) -> &str;

    /// Called every turn, after the event handlers.
    fn run(&mut self, status: &Status, cmd: &mut Commands);

    fn on_scanned_robot(&mut self, _event: &ScannedRobotEvent, _status: &Status, _cmd: &mut Commands) {}
    fn on_hit_by_bullet(&mut self, _event: &HitByBulletEvent, _status: &Status, _cmd: &mut Commands) {}
    fn on_bullet_hit(&mut self, _event: &BulletHitEvent, _status: &Status, _cmd: &mut Commands) {}
    fn on_bullet_hit_bullet(&mut self, _event: &BulletHitBulletEvent, _status: &Status, _cmd: &mut Commands) {}
    fn on_bullet_missed(&mut self, _event: &BulletMissedEvent, _status: &Status, _cmd: &mut Commands) {}
    fn on_hit_wall(&mut self, _event: &HitWallEvent, _status: &Status, _cmd: &mut Commands) {}
    fn on_hit_robot(&mut self, _event: &HitRobotEvent, _status: &Status, _cmd: &mut Commands) {}
    fn on_robot_death(&mut self, _event: &RobotDeathEvent, _status: &Status, _cmd: &mut Commands) {}

    /// You died (called right away, with your final status).
    fn on_death(&mut self, _status: &Status) {}
    /// You were the last robot standing when the round ended.
    fn on_win(&mut self, _status: &Status) {}
    /// The round ended. Robots keep their state between rounds, like static
    /// fields in Java robots.
    fn on_round_ended(&mut self, _event: &RoundEndedEvent) {}

    /// For fixed-point strategy robots: the observation and action of this
    /// turn, shown by the battle viewer. Ordinary robots leave this as `None`.
    fn last_decision(&self) -> Option<(Observation, Action)> {
        None
    }
}

/// `absolute` as seen from `heading`, in `(-180, 180]` degrees.
pub fn relative_bearing(absolute: f64, heading: f64) -> f64 {
    let mut d = (absolute - heading) % 360.0;
    if d > 180.0 {
        d -= 360.0;
    } else if d <= -180.0 {
        d += 360.0;
    }
    d
}
