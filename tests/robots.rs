use robocode::physics::{BattleConfig, SpawnSpec};
use robocode::robots::{SafeShooter, SittingDuck, Spinner, safe_shooter};
use robocode::runner::{Battle, boxed};
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

// Firing rules (when SafeShooter fires, and that every shot is safe) are
// proved by Verus; see docs/verification-receipt.md. These tests cover
// behaviour the spec leaves open: gun tracking, power choice, radar lock.

#[test]
fn keeps_tracking_a_stale_target_with_the_gun() {
    let o = Observation { target: Some(target(100, 300, 1)), ..obs() };
    assert_eq!(safe_shooter::decide(&o).turn_gun, Some(100));
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
    let mut battle = Battle::new(BattleConfig::default(), &spawns, vec![boxed(SafeShooter), boxed(SittingDuck)]);
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
    let outcome = duel(boxed(SittingDuck)).run(5000);
    assert_eq!(outcome.winner, Some(0));
    assert!(outcome.turns < 400, "took {} turns", outcome.turns);
}

#[test]
fn closest_target_picks_min_first_on_ties() {
    let ts = [target(0, 300, 0), target(0, 100, 0), target(0, 100, 0)];
    assert_eq!(closest_target(&ts), Some(1));
    assert_eq!(closest_target(&[]), None);
}

fn duel(opponent: Box<dyn robocode::api::Robot>) -> Battle {
    let spawns = [SpawnSpec::new(0, 100.0, 100.0, 0.0), SpawnSpec::new(1, 700.0, 500.0, 0.0)];
    Battle::new(BattleConfig::default(), &spawns, vec![boxed(SafeShooter), opponent])
}

#[test]
fn safe_shooter_beats_sitting_duck() {
    let outcome = duel(boxed(SittingDuck)).run(5000);
    assert_eq!(outcome.winner, Some(0), "{outcome:?}");
    assert_eq!(outcome.wall_hits[0], 0, "{outcome:?}");
}

#[test]
fn safe_shooter_avoids_walls_against_spinner() {
    let outcome = duel(boxed(Spinner)).run(5000);
    assert_eq!(outcome.wall_hits[0], 0, "{outcome:?}");
}

#[test]
fn battles_are_deterministic() {
    let a = duel(boxed(Spinner)).run(2000);
    let b = duel(boxed(Spinner)).run(2000);
    assert_eq!(a, b);
}

fn energies(o: &robocode::runner::BattleOutcome) -> String {
    format!("{:.1}/{:.1}", o.energies[0], o.energies[1])
}

#[test]
fn golden_safe_shooter_vs_spinner() {
    let o = duel(boxed(Spinner)).run(5000);
    assert_eq!((o.winner, o.turns, energies(&o)), (Some(0), 386, "77.0/-1.0".to_string()));
}

#[test]
fn golden_safe_shooter_vs_sitting_duck() {
    let o = duel(boxed(SittingDuck)).run(5000);
    assert_eq!((o.winner, o.turns, energies(&o)), (Some(0), 364, "147.0/0.0".to_string()));
}

// ---- Circler: a sample written only against robocode::api ----------------

use robocode::api::{Commands, HitWallEvent, Robot, Status};
use robocode::robots::Circler;

fn status() -> Status {
    Status {
        time: 5, name: "Circler".into(), others: 1, x: 400.0, y: 300.0, energy: 100.0, velocity: 8.0,
        gun_heat: 0.0, heading: 0.0, gun_heading: 0.0, radar_heading: 0.0, distance_remaining: 0.0,
        turn_remaining: 0.0, gun_turn_remaining: 0.0, radar_turn_remaining: 0.0,
        battlefield_width: 800.0, battlefield_height: 600.0, gun_cooling_rate: 0.1,
    }
}

#[test]
fn circler_reverses_on_wall() {
    let mut c = Circler::default();
    let mut cmd = Commands::default();
    c.run(&status(), &mut cmd);
    assert_eq!(cmd.ahead(), Some(100.0));
    let mut cmd = Commands::default();
    c.on_hit_wall(&HitWallEvent { bearing: 0.0 }, &status(), &mut cmd);
    c.run(&status(), &mut cmd);
    assert_eq!(cmd.ahead(), Some(-100.0));
}

#[test]
fn circler_beats_sitting_duck() {
    let spawns = [SpawnSpec::new(0, 100.0, 100.0, 0.0), SpawnSpec::new(1, 700.0, 500.0, 0.0)];
    let mut battle = Battle::new(BattleConfig::default(), &spawns, vec![Box::new(Circler::default()), boxed(SittingDuck)]);
    let outcome = battle.run(5000);
    assert_eq!(outcome.winner, Some(0), "{outcome:?}");
    assert_eq!(battle.name(0), "Circler");
}
