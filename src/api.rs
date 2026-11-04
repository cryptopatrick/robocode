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

/// Another robot died.
#[derive(Debug, Clone, PartialEq)]
pub struct RobotDeathEvent {
    pub name: String,
}

/// Something that happened last turn. Delivered in this order.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    ScannedRobot(ScannedRobotEvent),
    HitByBullet(HitByBulletEvent),
    BulletHit(BulletHitEvent),
    BulletMissed(BulletMissedEvent),
    HitWall(HitWallEvent),
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
    fn on_bullet_missed(&mut self, _event: &BulletMissedEvent, _status: &Status, _cmd: &mut Commands) {}
    fn on_hit_wall(&mut self, _event: &HitWallEvent, _status: &Status, _cmd: &mut Commands) {}
    fn on_robot_death(&mut self, _event: &RobotDeathEvent, _status: &Status, _cmd: &mut Commands) {}

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
