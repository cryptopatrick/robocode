use std::cell::Cell;
use std::rc::Rc;

use robocode::physics::{BattleConfig, RobotState, SpawnSpec};
use robocode::runner::{Battle, apply, observe};
use robocode::strategy::{Action, Observation, Strategy};
use robocode::units::{Angle, Energy, GunHeat, Power};
use robocode::{Battlefield, Point};

/// Returns a fixed action every turn and counts how often it was asked.
struct Scripted {
    action: Action,
    once: bool,
    calls: Rc<Cell<u32>>,
}

impl Scripted {
    fn new(action: Action) -> Self {
        Self { action, once: false, calls: Rc::new(Cell::new(0)) }
    }
}

impl Strategy for Scripted {
    fn name(&self) -> &str {
        "scripted"
    }
    fn decide(&mut self, _obs: &Observation) -> Action {
        self.calls.set(self.calls.get() + 1);
        if self.once && self.calls.get() > 1 { Action::default() } else { self.action }
    }
}

fn idle() -> Box<dyn Strategy> {
    Box::new(Scripted::new(Action::default()))
}

#[test]
fn observe_converts_units() {
    let mut r = RobotState::new(0, Point::new(100.4, 200.6));
    r.energy = 99.9996;
    r.heading = 359.96;
    r.gun_heat = 0.1 + 1e-16;
    let config = BattleConfig { battlefield: Battlefield::new(800.0, 600.0), ..BattleConfig::default() };
    let obs = observe(&r, &config, 7, None);
    assert_eq!(obs.energy, Energy(99_999));
    assert_eq!(obs.heading, Angle(0));
    // Gun heat is reported after this turn's cooling (0.1), as the engine sees
    // it when deciding whether to fire; float residue still rounds up.
    assert_eq!(obs.gun_heat, GunHeat(1));
    assert_eq!((obs.x, obs.y), (100, 201));
    assert_eq!((obs.field_width, obs.field_height), (800, 600));
    assert_eq!(obs.turn, 7);
    assert_eq!(obs.target, None);
}

#[test]
fn apply_sets_controls() {
    let mut r = RobotState::new(0, Point::new(100.0, 100.0));
    let action = Action { fire: Some(Power(1500)), turn_gun: Some(-150), ..Action::default() };
    apply(&mut r, &action);
    assert_eq!(r.control.fire_power, Some(1.5));
    assert_eq!(r.control.gun_turn_remaining, -15.0);
    assert_eq!(r.control.distance_remaining, 0.0);
}

#[test]
fn one_fire_one_bullet() {
    let mut shooter = Scripted::new(Action { fire: Some(Power(1000)), ..Action::default() });
    shooter.once = true;
    let spawns = [SpawnSpec::new(0, 100.0, 300.0, 0.0), SpawnSpec::new(1, 700.0, 300.0, 0.0)];
    let mut battle = Battle::new(BattleConfig::default(), &spawns, vec![Box::new(shooter), idle()]);
    let fired: usize = (0..30).map(|_| battle.step().bullet_fired.len()).sum();
    assert_eq!(fired, 1);
}

#[test]
fn last_survivor_wins() {
    // Robot 1 sits straight above robot 0, whose gun points up (0°).
    let shooter = Scripted::new(Action { fire: Some(Power(3000)), ..Action::default() });
    let spawns = [SpawnSpec::new(0, 400.0, 500.0, 0.0), SpawnSpec::new(1, 400.0, 200.0, 0.0)];
    let mut battle = Battle::new(BattleConfig::default(), &spawns, vec![Box::new(shooter), idle()]);
    battle.state_mut().robots[1].energy = 0.1;
    let outcome = battle.run(500);
    assert_eq!(outcome.winner, Some(0));
    assert!(outcome.turns < 500);
}

#[test]
fn simultaneous_death_has_no_winner() {
    // Both drive into opposite walls on the same turn with too little energy to survive.
    let ram = || -> Box<dyn Strategy> { Box::new(Scripted::new(Action { ahead: Some(100), ..Action::default() })) };
    let spawns = [SpawnSpec::new(0, 19.0, 300.0, 270.0), SpawnSpec::new(1, 781.0, 300.0, 90.0)];
    let mut battle = Battle::new(BattleConfig::default(), &spawns, vec![ram(), ram()]);
    battle.state_mut().robots[0].energy = 0.5;
    battle.state_mut().robots[1].energy = 0.5;
    let outcome = battle.run(100);
    assert_eq!(outcome.winner, None);
    assert!(outcome.turns < 100, "battle should end when both die, took {}", outcome.turns);
    assert_eq!(outcome.wall_hits, vec![1, 1]);
}

#[test]
fn dead_robots_are_not_asked_to_decide() {
    let doomed = Scripted::new(Action { ahead: Some(100), ..Action::default() });
    let calls = doomed.calls.clone();
    let spawns = [SpawnSpec::new(0, 19.0, 300.0, 270.0), SpawnSpec::new(1, 400.0, 300.0, 0.0)];
    let mut battle = Battle::new(BattleConfig::default(), &spawns, vec![Box::new(doomed), idle()]);
    battle.state_mut().robots[0].energy = 0.5;
    for _ in 0..10 {
        battle.step();
    }
    assert!(!battle.state().robots[0].alive);
    let at_death = calls.get();
    for _ in 0..10 {
        battle.step();
    }
    assert_eq!(calls.get(), at_death);
}

#[test]
fn decisions_are_recorded() {
    let action = Action { ahead: Some(50), turn_gun: Some(-150), ..Action::default() };
    let spawns = [SpawnSpec::new(0, 100.0, 300.0, 0.0), SpawnSpec::new(1, 700.0, 300.0, 0.0)];
    let mut battle = Battle::new(BattleConfig::default(), &spawns, vec![Box::new(Scripted::new(action)), idle()]);
    assert_eq!(battle.last_decision(1), None);
    battle.step();
    let (obs, recorded) = battle.last_decision(0).expect("robot 0 decided");
    assert_eq!(recorded, action);
    assert_eq!(obs.turn, 0);
    assert_eq!(obs.energy, Energy(100_000));
}

#[test]
fn dead_robot_has_no_decision() {
    let doomed = Scripted::new(Action { ahead: Some(100), ..Action::default() });
    let spawns = [SpawnSpec::new(0, 19.0, 300.0, 270.0), SpawnSpec::new(1, 400.0, 300.0, 0.0)];
    let mut battle = Battle::new(BattleConfig::default(), &spawns, vec![Box::new(doomed), idle()]);
    battle.state_mut().robots[0].energy = 0.5;
    for _ in 0..10 {
        battle.step();
    }
    assert!(!battle.state().robots[0].alive);
    assert_eq!(battle.last_decision(0), None);
    assert!(battle.last_decision(1).is_some());
}

#[test]
fn observed_cool_gun_matches_engine_firing() {
    // Cool a gun from every possible post-shot heat and compare, turn by turn,
    // "observation says cool" with "engine would fire" (heat <= 0 after cooling).
    let config = BattleConfig::default();
    for power in [0.1, 0.5, 1.0, 1.7, 3.0] {
        let mut heat = robocode::rules::gun_heat_for_power(power);
        for _ in 0..30 {
            let mut r = RobotState::new(0, Point::new(400.0, 300.0));
            r.gun_heat = heat;
            let obs = observe(&r, &config, 0, None);
            let engine_fires = (heat - config.gun_cooling_rate).max(0.0) <= 0.0;
            assert_eq!(obs.gun_heat == GunHeat(0), engine_fires, "power {power}, heat {heat:e}");
            heat = (heat - config.gun_cooling_rate).max(0.0);
        }
    }
}

#[test]
#[should_panic(expected = "battlefield must be at most 100000 px per side")]
fn rejects_battlefield_outside_the_proof() {
    // SafeShooter's proof assumes fields up to 100000 px (`well_formed`).
    let config = BattleConfig { battlefield: Battlefield::new(200_000.0, 600.0), ..BattleConfig::default() };
    let spawns = [SpawnSpec::new(0, 100.0, 100.0, 0.0), SpawnSpec::new(1, 500.0, 500.0, 0.0)];
    Battle::new(config, &spawns, vec![idle(), idle()]);
}

#[test]
#[should_panic(expected = "battlefield must be at most 100000 px per side")]
fn rejects_non_finite_battlefield() {
    let config = BattleConfig { battlefield: Battlefield::new(f64::NAN, 600.0), ..BattleConfig::default() };
    let spawns = [SpawnSpec::new(0, 100.0, 100.0, 0.0), SpawnSpec::new(1, 500.0, 500.0, 0.0)];
    Battle::new(config, &spawns, vec![idle(), idle()]);
}
