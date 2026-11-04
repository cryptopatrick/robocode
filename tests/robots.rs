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
    assert_eq!(at(1000), Some(Power(1000)));
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
}

#[test]
fn radar_always_spins() {
    assert_eq!(safe_shooter::decide(&obs()).turn_radar, Some(450));
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
