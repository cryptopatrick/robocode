use robocode::physics::{BattleConfig, SpawnSpec};
use robocode::robots::{SafeShooter, SittingDuck, Spinner, safe_shooter};
use robocode::runner::Battle;
use robocode::strategy::targeting::closest_target;
use robocode::strategy::{Observation, TargetInfo};
use robocode::units::{Angle, Energy, GunHeat, Power};

fn target(bearing: i32, distance: i32, age: u32) -> TargetInfo {
    TargetInfo { bearing: Angle(bearing), distance, energy: Energy(100_000), heading: Angle(0), velocity: 0, age }
}

/// A healthy robot in mid-field, gun at 0°, cool gun, target dead ahead at 300px.
fn obs() -> Observation {
    Observation {
        turn: 10,
        energy: Energy(50_000),
        gun_heat: GunHeat(0),
        heading: Angle(900),
        gun_heading: Angle(0),
        radar_heading: Angle(0),
        velocity: 0,
        x: 400,
        y: 300,
        field_width: 800,
        field_height: 600,
        target: Some(target(0, 300, 0)),
    }
}

#[test]
fn fires_when_safe() {
    assert_eq!(safe_shooter::decide(&obs()).fire, Some(Power(1000)));
}

#[test]
fn never_fires_when_hot() {
    let o = Observation { gun_heat: GunHeat(1), ..obs() };
    assert_eq!(safe_shooter::decide(&o).fire, None);
}

#[test]
fn never_fires_without_target() {
    let o = Observation { target: None, ..obs() };
    assert_eq!(safe_shooter::decide(&o).fire, None);
}

#[test]
fn never_fires_on_stale_target() {
    let o = Observation { target: Some(target(100, 300, 1)), ..obs() };
    let a = safe_shooter::decide(&o);
    assert_eq!(a.fire, None);
    assert_eq!(a.turn_gun, Some(100));
}

#[test]
fn aimed_across_zero() {
    let o = Observation { gun_heading: Angle(3595), target: Some(target(5, 300, 0)), ..obs() };
    assert!(safe_shooter::is_aimed(&o));
    assert_eq!(safe_shooter::decide(&o).fire, Some(Power(1000)));
}

#[test]
fn not_aimed_outside_tolerance() {
    let o = Observation { target: Some(target(safe_shooter::AIM_TOLERANCE + 1, 300, 0)), ..obs() };
    assert!(!safe_shooter::is_aimed(&o));
    assert_eq!(safe_shooter::decide(&o).fire, None);
}

#[test]
fn energy_boundary() {
    let at = |e| safe_shooter::decide(&Observation { energy: Energy(e), ..obs() }).fire;
    // Firing all remaining energy would leave the robot at 0, which the
    // engine treats as dead, so "safe" requires energy strictly above p.
    assert_eq!(at(1001), Some(Power(1000)));
    assert_eq!(at(1000), None);
    assert_eq!(at(999), None);
    assert_eq!(at(0), None);
}

#[test]
fn close_range_power_needs_energy() {
    let o = Observation { energy: Energy(2_000), target: Some(target(0, 100, 0)), ..obs() };
    match safe_shooter::decide(&o).fire {
        None => {}
        Some(p) => assert!(p.0 <= 2_000, "fired {p:?} with only 2.0 energy"),
    }
    let rich = Observation { target: Some(target(0, 100, 0)), ..obs() };
    assert_eq!(safe_shooter::decide(&rich).fire, Some(Power(3000)));
    let exactly_three = Observation { energy: Energy(3_000), target: Some(target(0, 100, 0)), ..obs() };
    assert_eq!(safe_shooter::decide(&exactly_three).fire, Some(Power(1000)));
}

#[test]
fn firing_never_kills_the_shooter() {
    // Energy exactly 1.0 in the engine: a "safe" shot must not leave it dead.
    use robocode::physics::{BattleConfig, SpawnSpec};
    let spawns = [SpawnSpec::new(0, 400.0, 500.0, 0.0), SpawnSpec::new(1, 400.0, 200.0, 0.0)];
    let mut battle = Battle::new(BattleConfig::default(), &spawns, vec![Box::new(SafeShooter), Box::new(SittingDuck)]);
    for _ in 0..40 {
        battle.state_mut().robots[0].energy = 1.0;
        battle.step();
        assert!(battle.state().robots[0].alive, "SafeShooter killed itself by firing");
    }
}

#[test]
fn radar_spins_without_a_fresh_target() {
    assert_eq!(safe_shooter::decide(&Observation { target: None, ..obs() }).turn_radar, Some(450));
    let stale = Observation { target: Some(target(100, 300, 1)), ..obs() };
    assert_eq!(safe_shooter::decide(&stale).turn_radar, Some(450));
}

#[test]
fn radar_locks_onto_a_fresh_target() {
    // Overshoot the target by its own offset so the next sweep crosses it again.
    let o = Observation { radar_heading: Angle(900), target: Some(target(1000, 300, 0)), ..obs() };
    assert_eq!(safe_shooter::decide(&o).turn_radar, Some(200));
    let behind = Observation { radar_heading: Angle(1100), target: Some(target(1000, 300, 0)), ..obs() };
    assert_eq!(safe_shooter::decide(&behind).turn_radar, Some(-200));
    let across_zero = Observation { radar_heading: Angle(3550), target: Some(target(50, 300, 0)), ..obs() };
    assert_eq!(safe_shooter::decide(&across_zero).turn_radar, Some(200));
    let far = Observation { radar_heading: Angle(0), target: Some(target(1700, 300, 0)), ..obs() };
    assert_eq!(safe_shooter::decide(&far).turn_radar, Some(450));
}

#[test]
fn radar_lock_fires_more_often() {
    // With the radar locked, SafeShooter finishes SittingDuck sooner than the
    // 461 turns it needed with a spinning radar. (Gun heat caps it near 300:
    // a 1.0 shot heats the gun for 12 turns and the duck needs 25 hits.)
    let outcome = duel(Box::new(SittingDuck)).run(5000);
    assert_eq!(outcome.winner, Some(0));
    assert!(outcome.turns < 400, "took {} turns", outcome.turns);
}

#[test]
fn closest_target_picks_min_first_on_ties() {
    let ts = [target(0, 300, 0), target(0, 100, 0), target(0, 100, 0)];
    assert_eq!(closest_target(&ts), Some(1));
    assert_eq!(closest_target(&[]), None);
}

fn duel(opponent: Box<dyn robocode::strategy::Strategy>) -> Battle {
    let spawns = [SpawnSpec::new(0, 100.0, 100.0, 0.0), SpawnSpec::new(1, 700.0, 500.0, 0.0)];
    Battle::new(BattleConfig::default(), &spawns, vec![Box::new(SafeShooter), opponent])
}

#[test]
fn safe_shooter_beats_sitting_duck() {
    let outcome = duel(Box::new(SittingDuck)).run(5000);
    assert_eq!(outcome.winner, Some(0), "{outcome:?}");
    assert_eq!(outcome.wall_hits[0], 0, "{outcome:?}");
}

#[test]
fn safe_shooter_avoids_walls_against_spinner() {
    let outcome = duel(Box::new(Spinner)).run(5000);
    assert_eq!(outcome.wall_hits[0], 0, "{outcome:?}");
}

#[test]
fn battles_are_deterministic() {
    let a = duel(Box::new(Spinner)).run(2000);
    let b = duel(Box::new(Spinner)).run(2000);
    assert_eq!(a, b);
}

fn energies(o: &robocode::runner::BattleOutcome) -> String {
    format!("{:.1}/{:.1}", o.energies[0], o.energies[1])
}

#[test]
fn golden_safe_shooter_vs_spinner() {
    let o = duel(Box::new(Spinner)).run(5000);
    assert_eq!((o.winner, o.turns, energies(&o)), (Some(0), 386, "77.0/-1.0".to_string()));
}

#[test]
fn golden_safe_shooter_vs_sitting_duck() {
    let o = duel(Box::new(SittingDuck)).run(5000);
    assert_eq!((o.winner, o.turns, energies(&o)), (Some(0), 364, "147.0/0.0".to_string()));
}
