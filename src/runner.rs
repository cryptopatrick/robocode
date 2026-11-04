//! Connects [`Strategy`] robots to the `f64` physics engine.
//!
//! Each turn the runner converts engine state into a fixed-point
//! [`Observation`], asks the strategy to `decide`, and applies the returned
//! [`Action`] as engine commands.

use crate::physics::{
    BattleConfig, BattleState, RobotId, RobotState, ScannedRobot, SpawnSpec,
    TurnEvents,
};
use crate::strategy::{Action, Observation, Strategy, TargetInfo};
use crate::units::{Angle, Energy, GunHeat};

/// Builds the fixed-point observation for `robot`. `last_scan` is the most
/// recent scan of a target together with its age in turns.
///
/// Gun heat is reported **after** this turn's cooling, because the engine
/// cools the gun before it checks whether a shot can be fired. So
/// `gun_heat == GunHeat(0)` exactly when the engine would accept a shot.
pub fn observe(
    robot: &RobotState,
    config: &BattleConfig,
    turn: u64,
    last_scan: Option<(&ScannedRobot, u32)>,
) -> Observation {
    let field = config.battlefield;
    let heat_when_firing = (robot.gun_heat - config.gun_cooling_rate).max(0.0);
    Observation {
        turn: turn.min(u32::MAX as u64) as u32,
        energy: Energy::from_f64(robot.energy),
        gun_heat: GunHeat::from_f64(heat_when_firing),
        heading: Angle::from_degrees(robot.heading),
        gun_heading: Angle::from_degrees(robot.gun_heading),
        radar_heading: Angle::from_degrees(robot.radar_heading),
        velocity: (robot.velocity * 10.0).round() as i32,
        x: robot.position.x.round() as i32,
        y: robot.position.y.round() as i32,
        field_width: field.width.round() as i32,
        field_height: field.height.round() as i32,
        target: last_scan.map(|(scan, age)| TargetInfo {
            bearing: Angle::from_degrees(scan.bearing),
            distance: scan.distance.round() as i32,
            energy: Energy::from_f64(scan.energy),
            heading: Angle::from_degrees(scan.heading),
            velocity: (scan.velocity * 10.0).round() as i32,
            age,
        }),
    }
}

/// Applies `action` to the robot's pending commands. `None` fields leave the
/// corresponding command unchanged.
pub fn apply(robot: &mut RobotState, action: &Action) {
    if let Some(power) = action.fire {
        robot.set_fire(power.to_f64());
    }
    if let Some(distance) = action.ahead {
        robot.set_ahead(distance as f64);
    }
    if let Some(turn) = action.turn_body {
        robot.set_turn_right(turn as f64 / 10.0);
    }
    if let Some(turn) = action.turn_gun {
        robot.set_turn_gun_right(turn as f64 / 10.0);
    }
    if let Some(turn) = action.turn_radar {
        robot.set_turn_radar_right(turn as f64 / 10.0);
    }
}

/// The result of [`Battle::run`].
#[derive(Debug, Clone, PartialEq)]
pub struct BattleOutcome {
    /// The sole survivor, or `None` if nobody or several robots survived.
    pub winner: Option<RobotId>,
    pub turns: u64,
    /// Final energy per robot, indexed by robot position in the battle.
    pub energies: Vec<f64>,
    /// Wall collisions per robot.
    pub wall_hits: Vec<u32>,
}

/// A battle between strategies. Robot `i` is driven by strategy `i`.
pub struct Battle {
    state: BattleState,
    strategies: Vec<Box<dyn Strategy>>,
    /// Latest scan per robot and the turn it happened on.
    last_scan: Vec<Option<(ScannedRobot, u64)>>,
    wall_hits: Vec<u32>,
    /// What each robot saw and decided in the most recent step.
    decisions: Vec<Option<(Observation, Action)>>,
}

impl Battle {
    /// Panics if `spawns` and `strategies` differ in length.
    pub fn new(
        config: BattleConfig,
        spawns: &[SpawnSpec],
        strategies: Vec<Box<dyn Strategy>>,
    ) -> Battle {
        assert_eq!(
            spawns.len(),
            strategies.len(),
            "one strategy per spawn is required"
        );
        let n = spawns.len();
        Battle {
            state: BattleState::from_spawns(config, spawns),
            strategies,
            last_scan: vec![None; n],
            wall_hits: vec![0; n],
            decisions: vec![None; n],
        }
    }

    pub fn state(&self) -> &BattleState {
        &self.state
    }

    /// Mutable engine access, e.g. for setting up test scenarios.
    pub fn state_mut(&mut self) -> &mut BattleState {
        &mut self.state
    }

    /// Runs one turn: observe → decide → apply for every living robot, then
    /// advance the engine.
    pub fn step(&mut self) -> TurnEvents {
        let config = self.state.config;
        let turn = self.state.turn;
        self.decisions.iter_mut().for_each(|d| *d = None);
        for i in 0..self.state.robots.len() {
            if !self.state.robots[i].alive {
                continue;
            }
            // Forget targets that have died since they were scanned.
            if let Some((scan, _)) = &self.last_scan[i] {
                let target_alive = self
                    .state
                    .robots
                    .iter()
                    .any(|r| r.id == scan.target && r.alive);
                if !target_alive {
                    self.last_scan[i] = None;
                }
            }
            let scan = self.last_scan[i]
                .as_ref()
                .map(|(s, seen)| (s, turn.saturating_sub(*seen).min(u32::MAX as u64) as u32));
            let obs = observe(&self.state.robots[i], &config, turn, scan);
            let action = self.strategies[i].decide(&obs);
            apply(&mut self.state.robots[i], &action);
            self.decisions[i] = Some((obs, action));
        }

        let events = self.state.step();
        let now = self.state.turn;
        for (i, robot) in self.state.robots.iter().enumerate() {
            let nearest = events
                .scans
                .iter()
                .filter(|s| s.scanner == robot.id)
                .min_by(|a, b| a.distance.total_cmp(&b.distance));
            if let Some(scan) = nearest {
                self.last_scan[i] = Some((scan.clone(), now));
            }
            self.wall_hits[i] += events
                .wall_collisions
                .iter()
                .filter(|w| w.robot_id == robot.id)
                .count() as u32;
        }
        events
    }

    /// Steps until at most one robot is alive or `max_turns` turns have run.
    pub fn run(&mut self, max_turns: u64) -> BattleOutcome {
        let mut turns = 0;
        while turns < max_turns && self.alive_count() > 1 {
            self.step();
            turns += 1;
        }
        let survivors: Vec<RobotId> = self
            .state
            .robots
            .iter()
            .filter(|r| r.alive)
            .map(|r| r.id)
            .collect();
        BattleOutcome {
            winner: if survivors.len() == 1 { Some(survivors[0]) } else { None },
            turns,
            energies: self.state.robots.iter().map(|r| r.energy).collect(),
            wall_hits: self.wall_hits.clone(),
        }
    }

    /// The observation and action robot `i` used in the most recent step, or
    /// `None` if it was dead then (or no step has run yet).
    pub fn last_decision(&self, i: usize) -> Option<(Observation, Action)> {
        self.decisions[i]
    }

    /// The name of the strategy driving robot `i`.
    pub fn name(&self, i: usize) -> &str {
        self.strategies[i].name()
    }

    fn alive_count(&self) -> usize {
        self.state.robots.iter().filter(|r| r.alive).count()
    }
}
