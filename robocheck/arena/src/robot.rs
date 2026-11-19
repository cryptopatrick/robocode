//! Runs a RoboCheck candidate (`decide(obs, env)`) as a robocode 0.5.0 robot,
//! and logs every decision with its full observation.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::rc::Rc;

use robocode::api::{
    Commands, Robot, RobotDeathEvent, ScannedRobotEvent, Status,
};
use robocode::runner::{commands_for, observe_status};
use robocode::strategy::{Action, Observation};
use robocode::terrain::Terrain;

use crate::surroundings::{Surroundings, clearance};

/// A candidate's decision function.
pub type Decide = fn(&Observation, &Surroundings) -> Action;

/// One turn of a candidate: what it saw and what it did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecisionRecord {
    pub turn: u32,
    pub obs: Observation,
    pub env: Surroundings,
    pub action: Action,
}

fn opt<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map_or_else(|| "null".to_string(), |v| v.to_string())
}

impl DecisionRecord {
    /// One JSON object, no newline: every `Observation` field (the full
    /// target), both clearances and the action, in fixed-point units.
    pub fn to_json_line(&self) -> String {
        let o = &self.obs;
        let target = o.target.map_or_else(
            || "null".to_string(),
            |t| {
                format!(
                    r#"{{"bearing":{},"distance":{},"energy":{},"heading":{},"velocity":{},"age":{}}}"#,
                    t.bearing.0, t.distance, t.energy.0, t.heading.0, t.velocity, t.age
                )
            },
        );
        let a = &self.action;
        let mut s = String::new();
        let _ = write!(
            s,
            r#"{{"turn":{},"obs":{{"turn":{},"energy":{},"gun_heat":{},"heading":{},"gun_heading":{},"radar_heading":{},"velocity":{},"x":{},"y":{},"field_width":{},"field_height":{},"target":{},"bullet_mode":"{}"}},"env":{{"clearance_ahead":{},"clearance_behind":{}}},"action":{{"fire":{},"ahead":{},"turn_body":{},"turn_gun":{},"turn_radar":{}}}}}"#,
            self.turn,
            o.turn,
            o.energy.0,
            o.gun_heat.0,
            o.heading.0,
            o.gun_heading.0,
            o.radar_heading.0,
            o.velocity,
            o.x,
            o.y,
            o.field_width,
            o.field_height,
            target,
            o.bullet_mode.name(),
            self.env.clearance_ahead,
            self.env.clearance_behind,
            opt(a.fire.map(|p| p.0)),
            opt(a.ahead),
            opt(a.turn_body),
            opt(a.turn_gun),
            opt(a.turn_radar),
        );
        s
    }
}

/// The decisions of one robot, readable after it has been boxed into a battle.
pub type DecisionLog = Rc<RefCell<Vec<DecisionRecord>>>;

/// A candidate as a robocode robot. Target tracking is the same as robocode
/// 0.5.0's `StrategyRobot` (nearest scan of the turn, forgotten when it dies),
/// so with an `env`-blind `decide` the battle is identical.
pub struct ArenaRobot {
    name: String,
    decide: Decide,
    terrain: Terrain,
    width: f64,
    height: f64,
    target: Option<(ScannedRobotEvent, u64)>,
    scanned_this_turn: bool,
    decision: Option<(Observation, Action)>,
    log: DecisionLog,
}

impl ArenaRobot {
    /// `terrain` must be the battle's terrain (an empty `Terrain` for the open arena).
    pub fn new(
        name: &str,
        decide: Decide,
        terrain: Terrain,
        width: f64,
        height: f64,
    ) -> Self {
        ArenaRobot {
            name: name.to_string(),
            decide,
            terrain,
            width,
            height,
            target: None,
            scanned_this_turn: false,
            decision: None,
            log: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// A handle to the decision log; keep it before boxing the robot.
    pub fn log(&self) -> DecisionLog {
        Rc::clone(&self.log)
    }
}

impl Robot for ArenaRobot {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_scanned_robot(
        &mut self,
        event: &ScannedRobotEvent,
        status: &Status,
        _cmd: &mut Commands,
    ) {
        if !self.scanned_this_turn {
            self.target = Some((event.clone(), status.time));
            self.scanned_this_turn = true;
        }
    }

    fn on_robot_death(
        &mut self,
        event: &RobotDeathEvent,
        _status: &Status,
        _cmd: &mut Commands,
    ) {
        if self.target.as_ref().is_some_and(|(t, _)| t.name == event.name) {
            self.target = None;
        }
    }

    fn run(&mut self, status: &Status, cmd: &mut Commands) {
        let target = self.target.as_ref().map(|(t, seen)| {
            (t, status.time.saturating_sub(*seen).min(u32::MAX as u64) as u32)
        });
        let obs = observe_status(status, target);
        let env = clearance(
            &self.terrain,
            self.width,
            self.height,
            status.x,
            status.y,
            status.heading,
        );
        let action = (self.decide)(&obs, &env);
        *cmd = commands_for(&action);
        self.decision = Some((obs, action));
        self.log.borrow_mut().push(DecisionRecord {
            turn: obs.turn,
            obs,
            env,
            action,
        });
        self.scanned_this_turn = false;
    }

    fn last_decision(&self) -> Option<(Observation, Action)> {
        self.decision
    }
}
