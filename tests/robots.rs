use robocode::physics::{BattleConfig, SpawnSpec};
use robocode::robots::{SafeShooter, SittingDuck, Spinner, safe_shooter};
use robocode::runner::{Battle, boxed};
use robocode::strategy::targeting::closest_target;
use robocode::strategy::{Observation, TargetInfo};
use robocode::units::{Angle, Energy, GunHeat, Power};

fn target(bearing: i32, distance: i32, age: u32) -> TargetInfo {
    TargetInfo {
        bearing: Angle(bearing),
        distance,
        energy: Energy(100_000),
        heading: Angle(0),
        velocity: 0,
        age,
    }
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
    let o = Observation {
        energy: Energy(2_000),
        target: Some(target(0, 100, 0)),
        ..obs()
    };
    match safe_shooter::decide(&o).fire {
        None => {}
        Some(p) => assert!(p.0 <= 2_000, "fired {p:?} with only 2.0 energy"),
    }
    let rich = Observation { target: Some(target(0, 100, 0)), ..obs() };
    assert_eq!(safe_shooter::decide(&rich).fire, Some(Power(3000)));
    let exactly_three = Observation {
        energy: Energy(3_000),
        target: Some(target(0, 100, 0)),
        ..obs()
    };
    assert_eq!(safe_shooter::decide(&exactly_three).fire, Some(Power(1000)));
}

#[test]
fn firing_never_kills_the_shooter() {
    // Energy exactly 1.0 in the engine: a "safe" shot must not leave it dead.
    use robocode::physics::{BattleConfig, SpawnSpec};
    let spawns = [
        SpawnSpec::new(0, 400.0, 500.0, 0.0),
        SpawnSpec::new(1, 400.0, 200.0, 0.0),
    ];
    let mut battle = Battle::new(
        BattleConfig::default(),
        &spawns,
        vec![boxed(SafeShooter), boxed(SittingDuck)],
    );
    for _ in 0..40 {
        battle.state_mut().robots[0].energy = 1.0;
        battle.step();
        assert!(
            battle.state().robots[0].alive,
            "SafeShooter killed itself by firing"
        );
    }
}

#[test]
fn radar_spins_without_a_fresh_target() {
    assert_eq!(
        safe_shooter::decide(&Observation { target: None, ..obs() })
            .turn_radar,
        Some(450)
    );
    let stale = Observation { target: Some(target(100, 300, 1)), ..obs() };
    assert_eq!(safe_shooter::decide(&stale).turn_radar, Some(450));
}

#[test]
fn radar_locks_onto_a_fresh_target() {
    // Overshoot the target by its own offset so the next sweep crosses it again.
    let o = Observation {
        radar_heading: Angle(900),
        target: Some(target(1000, 300, 0)),
        ..obs()
    };
    assert_eq!(safe_shooter::decide(&o).turn_radar, Some(200));
    let behind = Observation {
        radar_heading: Angle(1100),
        target: Some(target(1000, 300, 0)),
        ..obs()
    };
    assert_eq!(safe_shooter::decide(&behind).turn_radar, Some(-200));
    let across_zero = Observation {
        radar_heading: Angle(3550),
        target: Some(target(50, 300, 0)),
        ..obs()
    };
    assert_eq!(safe_shooter::decide(&across_zero).turn_radar, Some(200));
    let far = Observation {
        radar_heading: Angle(0),
        target: Some(target(1700, 300, 0)),
        ..obs()
    };
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
    let spawns = [
        SpawnSpec::new(0, 100.0, 100.0, 0.0),
        SpawnSpec::new(1, 700.0, 500.0, 0.0),
    ];
    Battle::new(
        BattleConfig::default(),
        &spawns,
        vec![boxed(SafeShooter), opponent],
    )
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
    assert_eq!(
        (o.winner, o.turns, energies(&o)),
        (Some(0), 386, "77.0/-1.0".to_string())
    );
}

#[test]
fn golden_safe_shooter_vs_sitting_duck() {
    let o = duel(boxed(SittingDuck)).run(5000);
    assert_eq!(
        (o.winner, o.turns, energies(&o)),
        (Some(0), 364, "147.0/0.0".to_string())
    );
}

// ---- Circler: a sample written only against robocode::api ----------------

use robocode::api::{Commands, HitWallEvent, Robot, Status};
use robocode::robots::Circler;

fn status() -> Status {
    Status {
        time: 5,
        name: "Circler".into(),
        others: 1,
        x: 400.0,
        y: 300.0,
        energy: 100.0,
        velocity: 8.0,
        gun_heat: 0.0,
        heading: 0.0,
        gun_heading: 0.0,
        radar_heading: 0.0,
        distance_remaining: 0.0,
        turn_remaining: 0.0,
        gun_turn_remaining: 0.0,
        radar_turn_remaining: 0.0,
        battlefield_width: 800.0,
        battlefield_height: 600.0,
        gun_cooling_rate: 0.1,
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
    let spawns = [
        SpawnSpec::new(0, 100.0, 100.0, 0.0),
        SpawnSpec::new(1, 700.0, 500.0, 0.0),
    ];
    let mut battle = Battle::new(
        BattleConfig::default(),
        &spawns,
        vec![Box::new(Circler::default()), boxed(SittingDuck)],
    );
    let outcome = battle.run(5000);
    assert_eq!(outcome.winner, Some(0), "{outcome:?}");
    assert_eq!(battle.name(0), "Circler");
}

// --- Tactician ---

mod tactician {
    use super::*;
    use robocode::recording;
    use robocode::robots::Tactician;
    use robocode::robots::tactician::choose;
    use robocode::roster;
    use robocode::strategy::Strategy;
    use robocode::strategy::maneuver::{Maneuver, Risk};
    use robocode::terrain::Terrain;

    fn with(energy: u32, t: Option<TargetInfo>) -> Observation {
        Observation { energy: Energy(energy), target: t, ..obs() }
    }

    #[test]
    fn choose_table() {
        use Maneuver::*;
        let n = Risk::Normal;
        let fresh = Some(target(0, 300, 0));
        let aged = Some(target(0, 300, 5));
        let stale = Some(target(0, 300, 25));
        let rows: Vec<(
            &str,
            Maneuver,
            u32,
            bool,
            Observation,
            Risk,
            Maneuver,
        )> = vec![
            ("shot evades", Attack, 20, true, with(50_000, fresh), n, Evade),
            (
                "evade interrupts early",
                Patrol,
                1,
                true,
                with(50_000, fresh),
                n,
                Evade,
            ),
            (
                "search when no target",
                Attack,
                20,
                false,
                with(50_000, None),
                n,
                Search,
            ),
            (
                "search when stale",
                Attack,
                20,
                false,
                with(50_000, stale),
                n,
                Search,
            ),
            (
                "search swaps to patrol at 150",
                Search,
                150,
                false,
                with(50_000, None),
                n,
                Patrol,
            ),
            (
                "search holds before 150",
                Search,
                149,
                false,
                with(50_000, None),
                n,
                Search,
            ),
            (
                "patrol holds before 150",
                Patrol,
                149,
                false,
                with(50_000, None),
                n,
                Patrol,
            ),
            (
                "patrol swaps to search at 150",
                Patrol,
                150,
                false,
                with(50_000, None),
                n,
                Search,
            ),
            (
                "search at 150 sees a fresh target",
                Search,
                150,
                false,
                with(50_000, fresh),
                n,
                Attack,
            ),
            (
                "defensive when low",
                Attack,
                20,
                false,
                with(10_000, fresh),
                n,
                Defensive,
            ),
            (
                "attack when fresh",
                Patrol,
                20,
                false,
                with(50_000, fresh),
                n,
                Attack,
            ),
            (
                "patrol when aged",
                Attack,
                20,
                false,
                with(50_000, aged),
                n,
                Patrol,
            ),
            (
                "hysteresis holds attack",
                Attack,
                3,
                false,
                with(50_000, stale),
                n,
                Attack,
            ),
            (
                "hysteresis ends at 8",
                Attack,
                8,
                false,
                with(50_000, stale),
                n,
                Search,
            ),
            (
                "aggressive ignores shot above reserve",
                Attack,
                20,
                true,
                with(50_000, fresh),
                Risk::Aggressive,
                Attack,
            ),
            (
                "aggressive evades below reserve",
                Attack,
                20,
                true,
                with(4_000, fresh),
                Risk::Aggressive,
                Evade,
            ),
            (
                "evade holds before evade_turns",
                Evade,
                11,
                false,
                with(50_000, fresh),
                n,
                Evade,
            ),
            (
                "evade ends after evade_turns",
                Evade,
                12,
                false,
                with(50_000, fresh),
                n,
                Attack,
            ),
        ];
        for (what, cur, turns, shot, o, risk, want) in rows {
            let got = choose(cur, turns, shot, &o, &risk.params(), risk);
            assert_eq!(got, want, "{what}");
        }
    }

    #[test]
    fn tactician_reports_its_intent() {
        let mut t = Tactician::new(Risk::Cautious, 3);
        t.decide(&obs());
        let i = t.intent().expect("Tactician has an intent");
        assert_eq!(i.risk, Risk::Cautious);
    }

    #[test]
    fn existing_robots_have_no_intent() {
        assert_eq!(boxed_strategy_intent(), None);
    }

    fn boxed_strategy_intent() -> Option<robocode::strategy::maneuver::Intent>
    {
        SafeShooter.intent()
    }

    fn battle(t: Tactician) -> Battle {
        let robots = vec![boxed(t), roster::built_in("Circler").unwrap()];
        let mut b = Battle::new(
            BattleConfig::default(),
            &roster::spawns(2, 800.0, 600.0),
            robots,
        );
        b.set_terrain(Terrain::urban(800.0, 600.0));
        b
    }

    #[test]
    fn tactician_finds_a_hidden_opponent() {
        let robots = vec![
            boxed(Tactician::default()),
            roster::built_in("SafeShooter").unwrap(),
        ];
        let mut b = Battle::new(
            BattleConfig::default(),
            &roster::spawns(2, 800.0, 600.0),
            robots,
        );
        b.set_terrain(Terrain::urban(800.0, 600.0));
        let mut seen = false;
        for _ in 0..1500 {
            b.step();
            if let Some((o, _)) = b.last_decision(0) {
                if o.target.is_some_and(|t| t.age == 0) {
                    seen = true;
                    break;
                }
            }
        }
        assert!(seen, "Tactician never saw SafeShooter in 1500 turns");
    }

    #[test]
    fn tactician_does_not_wall_bash() {
        let robots = vec![
            boxed(Tactician::default()),
            roster::built_in("SafeShooter").unwrap(),
        ];
        let mut b = Battle::new(
            BattleConfig::default(),
            &roster::spawns(2, 800.0, 600.0),
            robots,
        );
        b.set_terrain(Terrain::urban(800.0, 600.0));
        // Sum the damage of each wall hit, at the velocity of that turn.
        let mut damage = 0.0;
        for _ in 0..500 {
            let ev = b.step();
            let hit = ev.wall_collisions.iter().any(|w| w.robot_id == 0);
            if let (true, Some((o, _))) = (hit, b.last_decision(0)) {
                damage += robocode::rules::wall_hit_damage(o.velocity as f64 / 10.0);
            }
        }
        assert!(damage < 10.0, "Tactician took {damage} wall damage in 500 turns");
    }

    #[test]
    fn tactician_battles_are_deterministic() {
        for make in [(|| Tactician::default()) as fn() -> Tactician, || {
            Tactician::new(Risk::Aggressive, 9)
        }] {
            let a = recording::record(battle(make()), 500);
            let b = recording::record(battle(make()), 500);
            assert_eq!(a, b);
        }
    }
}
