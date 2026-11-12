//! Runs battles between [`Robot`]s on the `f64` physics engine.
//!
//! Each turn, every living robot gets its events from the previous turn, then
//! [`Robot::run`]; the [`Commands`] it set are applied and the engine steps.
//! Fixed-point [`Strategy`] robots (the ones RoboVerus proves things about)
//! take part through [`StrategyRobot`], which builds their [`Observation`].

use crate::api::{
    BulletHitBulletEvent, BulletHitEvent, BulletMissedEvent, Commands, RoundEndedEvent, SkippedTurnEvent, Event, HitByBulletEvent, HitRobotEvent, HitWallEvent, Robot,
    RobotDeathEvent, ScannedRobotEvent, Status, relative_bearing,
};
use crate::geometry::angle_to_point;
use crate::physics::{
    BattleConfig, BattleState, RobotId, RobotState, ScannedRobot, SpawnSpec, TurnEvents,
};
use crate::strategy::{Action, Observation, Strategy, TargetInfo};
use crate::units::{Angle, Energy, GunHeat};

/// The [`Status`] of `robot` as its strategy or handlers see it.
fn status_of(robot: &RobotState, config: &BattleConfig, turn: u64, name: &str, others: usize) -> Status {
    Status {
        time: turn,
        name: name.to_string(),
        others,
        x: robot.position.x,
        y: robot.position.y,
        energy: robot.energy,
        velocity: robot.velocity,
        gun_heat: robot.gun_heat,
        heading: robot.heading,
        gun_heading: robot.gun_heading,
        radar_heading: robot.radar_heading,
        distance_remaining: robot.control.distance_remaining,
        turn_remaining: robot.control.turn_remaining,
        gun_turn_remaining: robot.control.gun_turn_remaining,
        radar_turn_remaining: robot.control.radar_turn_remaining,
        battlefield_width: config.battlefield.width,
        battlefield_height: config.battlefield.height,
        gun_cooling_rate: config.gun_cooling_rate,
    }
}

fn scan_event(scan: &ScannedRobot, name: &str, heading: f64) -> ScannedRobotEvent {
    ScannedRobotEvent {
        name: name.to_string(),
        bearing: relative_bearing(scan.bearing, heading),
        absolute_bearing: scan.bearing,
        distance: scan.distance,
        energy: scan.energy,
        heading: scan.heading,
        velocity: scan.velocity,
    }
}

/// The fixed-point observation for a robot with this `status`. `target` is
/// the most recent scan together with its age in turns.
///
/// Gun heat is reported **after** this turn's cooling, because the engine
/// cools the gun before it checks whether a shot can be fired. So
/// `gun_heat == GunHeat(0)` exactly when the engine would accept a shot.
pub fn observe_status(status: &Status, target: Option<(&ScannedRobotEvent, u32)>) -> Observation {
    let heat_when_firing = (status.gun_heat - status.gun_cooling_rate).max(0.0);
    Observation {
        turn: status.time.min(u32::MAX as u64) as u32,
        energy: Energy::from_f64(status.energy),
        gun_heat: GunHeat::from_f64(heat_when_firing),
        heading: Angle::from_degrees(status.heading),
        gun_heading: Angle::from_degrees(status.gun_heading),
        radar_heading: Angle::from_degrees(status.radar_heading),
        velocity: (status.velocity * 10.0).round() as i32,
        x: status.x.round() as i32,
        y: status.y.round() as i32,
        field_width: status.battlefield_width.round() as i32,
        field_height: status.battlefield_height.round() as i32,
        target: target.map(|(scan, age)| TargetInfo {
            bearing: Angle::from_degrees(scan.absolute_bearing),
            distance: scan.distance.round() as i32,
            energy: Energy::from_f64(scan.energy),
            heading: Angle::from_degrees(scan.heading),
            velocity: (scan.velocity * 10.0).round() as i32,
            age,
        }),
    }
}

/// [`observe_status`] straight from engine state.
pub fn observe(
    robot: &RobotState,
    config: &BattleConfig,
    turn: u64,
    last_scan: Option<(&ScannedRobot, u32)>,
) -> Observation {
    let status = status_of(robot, config, turn, "", 0);
    let event = last_scan.map(|(scan, age)| (scan_event(scan, "", robot.heading), age));
    observe_status(&status, event.as_ref().map(|(e, age)| (e, *age)))
}

/// The engine commands for a fixed-point `action`.
pub fn commands_for(action: &Action) -> Commands {
    let mut cmd = Commands::default();
    if let Some(power) = action.fire {
        cmd.set_fire(power.to_f64());
    }
    if let Some(distance) = action.ahead {
        cmd.set_ahead(distance as f64);
    }
    if let Some(turn) = action.turn_body {
        cmd.set_turn_right(turn as f64 / 10.0);
    }
    if let Some(turn) = action.turn_gun {
        cmd.set_turn_gun_right(turn as f64 / 10.0);
    }
    if let Some(turn) = action.turn_radar {
        cmd.set_turn_radar_right(turn as f64 / 10.0);
    }
    cmd
}

/// Applies `action` to the robot's pending commands. `None` fields leave the
/// corresponding command unchanged.
pub fn apply(robot: &mut RobotState, action: &Action) {
    commands_for(action).apply_to(robot);
}

/// Runs a fixed-point [`Strategy`] as a [`Robot`]: it remembers the nearest
/// robot scanned (and how many turns ago), forgets it when it dies, and
/// hands `decide` the resulting [`Observation`].
pub struct StrategyRobot<S: Strategy> {
    strategy: S,
    name: String,
    target: Option<(ScannedRobotEvent, u64)>,
    scanned_this_turn: bool,
    decision: Option<(Observation, Action)>,
}

impl<S: Strategy> StrategyRobot<S> {
    pub fn new(strategy: S) -> Self {
        let name = strategy.name().to_string();
        StrategyRobot { strategy, name, target: None, scanned_this_turn: false, decision: None }
    }
}

impl<S: Strategy> Robot for StrategyRobot<S> {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_scanned_robot(&mut self, event: &ScannedRobotEvent, status: &Status, _cmd: &mut Commands) {
        // Scans arrive nearest first; keep the nearest of this turn.
        if !self.scanned_this_turn {
            self.target = Some((event.clone(), status.time));
            self.scanned_this_turn = true;
        }
    }

    fn on_robot_death(&mut self, event: &RobotDeathEvent, _status: &Status, _cmd: &mut Commands) {
        if self.target.as_ref().is_some_and(|(t, _)| t.name == event.name) {
            self.target = None;
        }
    }

    fn run(&mut self, status: &Status, cmd: &mut Commands) {
        let target = self
            .target
            .as_ref()
            .map(|(t, seen)| (t, status.time.saturating_sub(*seen).min(u32::MAX as u64) as u32));
        let obs = observe_status(status, target);
        debug_assert!(crate::strategy::is_well_formed(&obs), "ill-formed observation: {obs:?}");
        let action = self.strategy.decide(&obs);
        *cmd = commands_for(&action);
        self.decision = Some((obs, action));
        self.scanned_this_turn = false;
    }

    fn last_decision(&self) -> Option<(Observation, Action)> {
        self.decision
    }
}

/// Wraps a [`Strategy`] for [`Battle::new`].
pub fn boxed<S: Strategy + 'static>(strategy: S) -> Box<dyn Robot> {
    Box::new(StrategyRobot::new(strategy))
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

/// A battle between robots. Robot `i` spawns at `spawns[i]`.
pub struct Battle {
    state: BattleState,
    robots: Vec<Box<dyn Robot>>,
    names: Vec<String>,
    /// Events waiting to be delivered to each robot next turn.
    pending: Vec<Vec<Event>>,
    /// Whether each robot acted in the most recent step.
    acted: Vec<bool>,
    wall_hits: Vec<u32>,
    /// Where each robot spawned in round 0 (later rounds rotate these).
    spawns: Vec<SpawnSpec>,
    /// Bullet damage dealt by each robot since the last `run_rounds` began.
    bullet_damage: Vec<f64>,
    /// Score this round, and totals over finished rounds.
    round_scores: Vec<Score>,
    scores: Vec<Score>,
    /// `damage_to[i][j]`: damage robot `i` dealt robot `j` this round (for kill bonuses).
    damage_to: Vec<Vec<f64>>,
    /// Per-robot time budget per turn (Java-style); `None` = unlimited.
    time_budget: Option<std::time::Duration>,
    /// Turns each robot still has to skip, how many in a row it has skipped,
    /// how many it hasn't been told about, and its total.
    skip_left: Vec<u32>,
    skip_streak: Vec<u32>,
    skip_unreported: Vec<u32>,
    skipped: Vec<u32>,
}

/// Robocode's score, as Java's `RobotStatistics` computes it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Score {
    /// +50 each time another robot dies while this one is alive.
    pub survival: f64,
    /// +10 per opponent for being the last robot standing.
    pub last_survivor_bonus: f64,
    /// Bullet damage dealt, capped at each victim's remaining energy.
    pub bullet_damage: f64,
    /// 20% of all damage dealt to a robot this one killed with a bullet.
    pub bullet_kill_bonus: f64,
    /// +1.2 (2 × ram damage) per collision this robot caused.
    pub ram_damage: f64,
    /// 30% of all damage dealt to a robot this one rammed to death.
    pub ram_kill_bonus: f64,
}

impl Score {
    pub fn total(&self) -> f64 {
        self.survival + self.last_survivor_bonus + self.bullet_damage + self.bullet_kill_bonus + self.ram_damage + self.ram_kill_bonus
    }

    fn add(&mut self, other: &Score) {
        self.survival += other.survival;
        self.last_survivor_bonus += other.last_survivor_bonus;
        self.bullet_damage += other.bullet_damage;
        self.bullet_kill_bonus += other.bullet_kill_bonus;
        self.ram_damage += other.ram_damage;
        self.ram_kill_bonus += other.ram_kill_bonus;
    }
}

/// Results of [`Battle::run_rounds`].
#[derive(Debug, Clone, PartialEq)]
pub struct BattleResults {
    /// One outcome per round.
    pub rounds: Vec<BattleOutcome>,
    /// Rounds won by each robot (last one standing).
    pub wins: Vec<u32>,
    /// Rounds each robot was still alive at the end of.
    pub survived: Vec<u32>,
    /// Total bullet damage each robot dealt.
    pub bullet_damage: Vec<f64>,
    /// Robocode score per robot, summed over all rounds.
    pub scores: Vec<Score>,
    /// Turns each robot skipped for exceeding the time budget (0 without one).
    pub skipped_turns: Vec<u32>,
}

/// Names as classic RoboCode shows them: repeats get " (2)", " (3)", …
fn unique_names(robots: &[Box<dyn Robot>]) -> Vec<String> {
    robots
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let name = r.name();
            let earlier = robots[..i].iter().filter(|o| o.name() == name).count();
            if earlier == 0 { name.to_string() } else { format!("{name} ({})", earlier + 1) }
        })
        .collect()
}

/// Bearing from a robot to the wall it hit, in absolute degrees.
fn wall_bearing(robot: &RobotState, config: &BattleConfig) -> f64 {
    let (x, y, r) = (robot.position.x, robot.position.y, robot.radius);
    let field = config.battlefield;
    if x <= r {
        270.0
    } else if x >= field.width - r {
        90.0
    } else if y <= r {
        0.0
    } else if y >= field.height - r {
        180.0
    } else {
        robot.heading
    }
}

impl Battle {
    /// Largest battlefield side the strategy proofs cover (`well_formed` in
    /// `robots::safe_shooter`).
    pub const MAX_FIELD_SIDE: f64 = 100_000.0;

    /// Panics if `spawns` and `robots` differ in length, or if the
    /// battlefield is not between 1 and [`Self::MAX_FIELD_SIDE`] px per side.
    pub fn new(config: BattleConfig, spawns: &[SpawnSpec], robots: Vec<Box<dyn Robot>>) -> Battle {
        assert_eq!(spawns.len(), robots.len(), "one robot per spawn is required");
        let side_ok = |s: f64| (1.0..=Self::MAX_FIELD_SIDE).contains(&s);
        assert!(
            side_ok(config.battlefield.width) && side_ok(config.battlefield.height),
            "battlefield must be at most 100000 px per side (and at least 1)"
        );
        let n = spawns.len();
        Battle {
            state: BattleState::from_spawns(config, spawns),
            names: unique_names(&robots),
            robots,
            pending: vec![Vec::new(); n],
            acted: vec![false; n],
            wall_hits: vec![0; n],
            spawns: spawns.to_vec(),
            bullet_damage: vec![0.0; n],
            round_scores: vec![Score::default(); n],
            scores: vec![Score::default(); n],
            damage_to: vec![vec![0.0; n]; n],
            time_budget: None,
            skip_left: vec![0; n],
            skip_streak: vec![0; n],
            skip_unreported: vec![0; n],
            skipped: vec![0; n],
        }
    }

    pub fn state(&self) -> &BattleState {
        &self.state
    }

    /// Mutable engine access, e.g. for setting up test scenarios.
    pub fn state_mut(&mut self) -> &mut BattleState {
        &mut self.state
    }

    /// Runs one turn: events and `run` for every living robot, then the
    /// engine step.
    pub fn step(&mut self) -> TurnEvents {
        let config = self.state.config;
        let turn = self.state.turn;
        let alive = self.alive_count();
        for i in 0..self.robots.len() {
            let events = std::mem::take(&mut self.pending[i]);
            self.acted[i] = false;
            if !self.state.robots[i].alive {
                continue;
            }
            if self.skip_left[i] > 0 {
                // Still "busy" from an over-budget turn: no events, no run;
                // events wait for the next turn, pending commands keep executing.
                self.skip_left[i] -= 1;
                self.skip_streak[i] += 1;
                self.skip_unreported[i] += 1;
                self.skipped[i] += 1;
                self.pending[i] = events;
                if self.skip_streak[i] >= Self::MAX_SKIPPED_TURNS {
                    self.state.robots[i].energy = 0.0; // disabled, as in Java
                }
                continue;
            }
            self.skip_streak[i] = 0;
            let started = self.start_timer();
            let status = status_of(&self.state.robots[i], &config, turn, &self.names[i], alive - 1);
            let mut cmd = Commands::default();
            let robot = &mut self.robots[i];
            if self.skip_unreported[i] > 0 {
                let event = SkippedTurnEvent { turns: std::mem::take(&mut self.skip_unreported[i]) };
                robot.on_skipped_turn(&event, &status, &mut cmd);
            }
            for event in &events {
                match event {
                    Event::ScannedRobot(e) => robot.on_scanned_robot(e, &status, &mut cmd),
                    Event::HitByBullet(e) => robot.on_hit_by_bullet(e, &status, &mut cmd),
                    Event::BulletHit(e) => robot.on_bullet_hit(e, &status, &mut cmd),
                    Event::BulletHitBullet(e) => robot.on_bullet_hit_bullet(e, &status, &mut cmd),
                    Event::BulletMissed(e) => robot.on_bullet_missed(e, &status, &mut cmd),
                    Event::HitWall(e) => robot.on_hit_wall(e, &status, &mut cmd),
                    Event::HitRobot(e) => robot.on_hit_robot(e, &status, &mut cmd),
                    Event::RobotDeath(e) => robot.on_robot_death(e, &status, &mut cmd),
                }
            }
            robot.run(&status, &mut cmd);
            cmd.apply_to(&mut self.state.robots[i]);
            self.acted[i] = true;
            self.skip_left[i] = self.turns_over_budget(started);
        }

        let events = self.state.step();
        self.queue_events(&events);
        let config = self.state.config;
        let turn = self.state.turn;
        let alive = self.alive_count();
        for hit in &events.bullet_hits {
            if let Some(s) = self.state.robots.iter().position(|r| r.id == hit.shooter) {
                self.bullet_damage[s] += crate::rules::bullet_damage(hit.power);
            }
        }
        self.score_turn(&events);
        for &dead in &events.robot_deaths {
            if let Some(d) = self.state.robots.iter().position(|r| r.id == dead) {
                let status = status_of(&self.state.robots[d], &config, turn, &self.names[d], alive);
                self.robots[d].on_death(&status);
            }
        }
        events
    }

    /// Turns engine events into per-robot API events for next turn, in
    /// [`Event`] order.
    fn queue_events(&mut self, events: &TurnEvents) {
        let config = self.state.config;
        let index = |id: RobotId| self.state.robots.iter().position(|r| r.id == id);
        let mut pending: Vec<Vec<Event>> = vec![Vec::new(); self.robots.len()];

        for (i, robot) in self.state.robots.iter().enumerate() {
            let mut scans: Vec<&ScannedRobot> = events.scans.iter().filter(|s| s.scanner == robot.id).collect();
            scans.sort_by(|a, b| a.distance.total_cmp(&b.distance));
            for scan in scans {
                if let Some(t) = index(scan.target) {
                    pending[i].push(Event::ScannedRobot(scan_event(scan, &self.names[t], robot.heading)));
                }
            }
        }
        for hit in &events.bullet_hits {
            if let (Some(v), Some(s)) = (index(hit.victim), index(hit.shooter)) {
                let victim = &self.state.robots[v];
                let shooter = &self.state.robots[s];
                let bearing = relative_bearing(angle_to_point(victim.position, shooter.position), victim.heading);
                pending[v].push(Event::HitByBullet(HitByBulletEvent {
                    name: self.names[s].clone(),
                    bearing,
                    power: hit.power,
                }));
            }
        }
        for hit in &events.bullet_hits {
            if let (Some(v), Some(s)) = (index(hit.victim), index(hit.shooter)) {
                pending[s].push(Event::BulletHit(BulletHitEvent {
                    name: self.names[v].clone(),
                    energy: self.state.robots[v].energy,
                    power: hit.power,
                }));
            }
        }
        for c in &events.bullet_hit_bullets {
            if let (Some(a), Some(b)) = (index(c.owner), index(c.other_owner)) {
                pending[a].push(Event::BulletHitBullet(BulletHitBulletEvent {
                    name: self.names[b].clone(),
                    power: c.power,
                    other_power: c.other_power,
                }));
                pending[b].push(Event::BulletHitBullet(BulletHitBulletEvent {
                    name: self.names[a].clone(),
                    power: c.other_power,
                    other_power: c.power,
                }));
            }
        }
        for miss in &events.bullet_misses {
            if let Some(s) = index(miss.shooter) {
                pending[s].push(Event::BulletMissed(BulletMissedEvent { power: miss.power }));
            }
        }
        for wall in &events.wall_collisions {
            if let Some(i) = index(wall.robot_id) {
                let robot = &self.state.robots[i];
                let bearing = relative_bearing(wall_bearing(robot, &config), robot.heading);
                pending[i].push(Event::HitWall(HitWallEvent { bearing }));
            }
        }
        for c in &events.robot_collisions {
            if let (Some(i), Some(o)) = (index(c.robot_id), index(c.other_robot_id)) {
                let (me, other) = (&self.state.robots[i], &self.state.robots[o]);
                let bearing = relative_bearing(angle_to_point(me.position, other.position), me.heading);
                pending[i].push(Event::HitRobot(HitRobotEvent {
                    name: self.names[o].clone(),
                    bearing,
                    energy: other.energy,
                    at_fault: c.at_fault,
                }));
            }
        }
        for &dead in &events.robot_deaths {
            if let Some(d) = index(dead) {
                for (i, list) in pending.iter_mut().enumerate() {
                    if i != d {
                        list.push(Event::RobotDeath(RobotDeathEvent { name: self.names[d].clone() }));
                    }
                }
            }
        }

        for (i, robot) in self.state.robots.iter().enumerate() {
            self.wall_hits[i] +=
                events.wall_collisions.iter().filter(|w| w.robot_id == robot.id).count() as u32;
        }
        for (queue, new) in self.pending.iter_mut().zip(pending) {
            queue.extend(new);
        }
    }

    /// Steps until at most one robot is alive or `max_turns` turns have run.
    pub fn run(&mut self, max_turns: u64) -> BattleOutcome {
        let outcome = self.play(max_turns);
        self.finish_round(0, 1, &outcome);
        outcome
    }

    /// Plays `rounds` rounds of at most `max_turns_per_round` turns each.
    /// Each round starts fresh, with spawn points rotated by the round number
    /// so every robot gets every corner; robot instances keep their state.
    /// Call it on a fresh battle.
    pub fn run_rounds(&mut self, rounds: u32, max_turns_per_round: u64) -> BattleResults {
        let n = self.robots.len();
        self.bullet_damage = vec![0.0; n];
        self.scores = vec![Score::default(); n];
        self.skipped = vec![0; n];
        let mut results = BattleResults {
            rounds: Vec::new(),
            wins: vec![0; n],
            survived: vec![0; n],
            bullet_damage: Vec::new(),
            scores: Vec::new(),
            skipped_turns: Vec::new(),
        };
        for round in 0..rounds {
            if round > 0 {
                self.reset_round(round);
            }
            let outcome = self.play(max_turns_per_round);
            self.finish_round(round, rounds, &outcome);
            if let Some(w) = outcome.winner.and_then(|id| self.state.robots.iter().position(|r| r.id == id)) {
                results.wins[w] += 1;
            }
            for (i, r) in self.state.robots.iter().enumerate() {
                if r.alive {
                    results.survived[i] += 1;
                }
            }
            results.rounds.push(outcome);
        }
        results.bullet_damage = self.bullet_damage.clone();
        results.scores = self.scores.clone();
        results.skipped_turns = self.skipped.clone();
        results
    }

    /// Steps until at most one robot is alive or `max_turns` turns have run.
    fn play(&mut self, max_turns: u64) -> BattleOutcome {
        let mut turns = 0;
        while turns < max_turns && self.alive_count() > 1 {
            self.step();
            turns += 1;
        }
        self.current_outcome(turns)
    }

    /// The outcome as things stand, without ending the round (no callbacks).
    /// `turns` is reported as given.
    pub fn current_outcome(&self, turns: u64) -> BattleOutcome {
        let survivors: Vec<RobotId> = self.state.robots.iter().filter(|r| r.alive).map(|r| r.id).collect();
        BattleOutcome {
            winner: if survivors.len() == 1 { Some(survivors[0]) } else { None },
            turns,
            energies: self.state.robots.iter().map(|r| r.energy).collect(),
            wall_hits: self.wall_hits.clone(),
        }
    }

    /// Java's per-turn scoring: damage, ramming, kill bonuses, survival.
    fn score_turn(&mut self, events: &TurnEvents) {
        let index = |id: RobotId| self.state.robots.iter().position(|r| r.id == id);
        for hit in &events.bullet_hits {
            if let (Some(s), Some(v)) = (index(hit.shooter), index(hit.victim)) {
                let damage = crate::rules::bullet_damage(hit.power);
                // Java caps the score at the victim's energy before the hit.
                let before = self.state.robots[v].energy + damage;
                let score = damage.min(before.max(0.0));
                self.round_scores[s].bullet_damage += score;
                self.damage_to[s][v] += score;
            }
        }
        for c in events.robot_collisions.iter().filter(|c| c.at_fault) {
            if let (Some(a), Some(o)) = (index(c.robot_id), index(c.other_robot_id)) {
                self.round_scores[a].ram_damage += crate::rules::ROBOT_HIT_BONUS;
                self.damage_to[a][o] += crate::rules::ROBOT_HIT_DAMAGE;
            }
        }
        for &dead in &events.robot_deaths {
            let Some(d) = index(dead) else { continue };
            // Kill bonus to whoever dealt the fatal blow this turn.
            let fatal_hit = events.bullet_hits.iter().rev().find(|h| h.victim == dead);
            let fatal_ram = events.robot_collisions.iter().rev().find(|c| c.at_fault && c.other_robot_id == dead);
            if let Some(hit) = fatal_hit
                && let Some(s) = index(hit.shooter)
            {
                self.round_scores[s].bullet_kill_bonus += 0.2 * self.damage_to[s][d];
            } else if fatal_hit.is_none()
                && let Some(c) = fatal_ram
                && let Some(a) = index(c.robot_id)
            {
                self.round_scores[a].ram_kill_bonus += 0.3 * self.damage_to[a][d];
            }
            for (i, r) in self.state.robots.iter().enumerate() {
                if r.alive {
                    self.round_scores[i].survival += 50.0;
                }
            }
        }
    }

    /// Consecutive skipped turns after which a robot is disabled (Java's rule).
    pub const MAX_SKIPPED_TURNS: u32 = 30;

    /// Give each robot at most `budget` per turn (handlers plus `run`), like
    /// Java's CPU constant. A turn taking `k` whole budgets skips the robot's
    /// next `k` turns; 30 in a row disable it. `None` (the default) keeps
    /// battles fully deterministic. There is no preemption: a robot that never
    /// returns still hangs the battle. Has no effect on `wasm32`, which has no clock.
    /// Map objects for this battle. The default is an empty arena.
    pub fn set_terrain(&mut self, terrain: crate::terrain::Terrain) {
        self.state.terrain = terrain;
    }

    pub fn set_time_budget(&mut self, budget: Option<std::time::Duration>) {
        self.time_budget = budget;
    }

    /// Turns each robot has skipped for exceeding the time budget.
    pub fn skipped_turns(&self) -> &[u32] {
        &self.skipped
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn start_timer(&self) -> Option<std::time::Instant> {
        self.time_budget.map(|_| std::time::Instant::now())
    }

    #[cfg(target_arch = "wasm32")]
    fn start_timer(&self) -> Option<()> {
        None
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn turns_over_budget(&self, started: Option<std::time::Instant>) -> u32 {
        match (self.time_budget, started) {
            (Some(budget), Some(t)) if !budget.is_zero() => (t.elapsed().as_nanos() / budget.as_nanos()) as u32,
            _ => 0,
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn turns_over_budget(&self, _started: Option<()>) -> u32 {
        0
    }

    /// Robocode scores so far: finished rounds plus the current one.
    pub fn scores(&self) -> &[Score] {
        &self.scores
    }

    /// End-of-round callbacks: `on_win` for a sole survivor, then
    /// `on_round_ended` for everyone.
    fn finish_round(&mut self, round: u32, total_rounds: u32, outcome: &BattleOutcome) {
        let config = self.state.config;
        let turn = self.state.turn;
        if let Some(w) = outcome.winner.and_then(|id| self.state.robots.iter().position(|r| r.id == id)) {
            self.round_scores[w].last_survivor_bonus += 10.0 * (self.robots.len() - 1) as f64;
            let status = status_of(&self.state.robots[w], &config, turn, &self.names[w], 0);
            self.robots[w].on_win(&status);
        }
        let n = self.robots.len();
        for (total, round) in self.scores.iter_mut().zip(&self.round_scores) {
            total.add(round);
        }
        self.round_scores = vec![Score::default(); n];
        self.damage_to = vec![vec![0.0; n]; n];
        let event = RoundEndedEvent { round, total_rounds, turns: outcome.turns };
        for robot in &mut self.robots {
            robot.on_round_ended(&event);
        }
    }

    /// Fresh engine state for `round`: robot `i` spawns where robot
    /// `(i + round) % n` spawned in round 0.
    fn reset_round(&mut self, round: u32) {
        let n = self.spawns.len();
        let rotated: Vec<SpawnSpec> = (0..n)
            .map(|i| {
                let s = self.spawns[(i + round as usize) % n];
                SpawnSpec { id: self.spawns[i].id, ..s }
            })
            .collect();
        self.state = BattleState::from_spawns(self.state.config, &rotated);
        self.pending = vec![Vec::new(); n];
        self.acted = vec![false; n];
        self.wall_hits = vec![0; n];
        self.skip_left = vec![0; n];
        self.skip_streak = vec![0; n];
        self.skip_unreported = vec![0; n];
    }

    /// The observation and action robot `i` used in the most recent step, if
    /// it acted then and is a fixed-point strategy robot.
    pub fn last_decision(&self, i: usize) -> Option<(Observation, Action)> {
        if self.acted[i] { self.robots[i].last_decision() } else { None }
    }

    /// Robot `i`'s name in this battle (numbered if several share it).
    pub fn name(&self, i: usize) -> &str {
        &self.names[i]
    }

    fn alive_count(&self) -> usize {
        self.state.robots.iter().filter(|r| r.alive).count()
    }
}
