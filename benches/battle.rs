//! Benchmarks: how many turns per second the engine simulates.
//!
//! `cargo bench -p robocode` runs the full cases; `cargo bench -p robocode --
//! --test` runs a short version (CI uses this to keep the benchmark working).
//! There is no external harness, so this adds no dependencies.

use std::hint::black_box;
use std::time::Instant;

use robocode::physics::BattleConfig;
use robocode::runner::Battle;
use robocode::terrain::Terrain;
use robocode::{Battlefield, roster};

struct Case {
    name: &'static str,
    robots: &'static [&'static str],
    urban: bool,
}

const CASES: [Case; 4] = [
    Case {
        name: "two robots, open arena",
        robots: &["SafeShooter", "Spinner"],
        urban: false,
    },
    Case {
        name: "four robots, open arena",
        robots: &["SafeShooter", "Circler", "Spinner", "SittingDuck"],
        urban: false,
    },
    Case {
        name: "two robots, urban map",
        robots: &["SafeShooter", "Spinner"],
        urban: true,
    },
    Case {
        name: "four robots, urban map",
        robots: &["SafeShooter", "Circler", "Spinner", "SittingDuck"],
        urban: true,
    },
];

fn build(case: &Case, width: f64, height: f64) -> Battle {
    let robots = case
        .robots
        .iter()
        .map(|n| roster::built_in(n).expect("built-in"))
        .collect();
    let config = BattleConfig {
        battlefield: Battlefield::new(width, height),
        ..BattleConfig::default()
    };
    let mut battle = Battle::new(
        config,
        &roster::spawns(case.robots.len(), width, height),
        robots,
    );
    if case.urban {
        battle.set_terrain(Terrain::urban(width, height));
    }
    battle
}

fn main() {
    let quick = std::env::args().any(|a| a == "--test");
    let turns: u64 = if quick { 200 } else { 5000 };
    println!("{:<28} {:>10} {:>14}", "case", "turns", "turns/second");
    for case in &CASES {
        let mut battle = build(case, 800.0, 600.0);
        let start = Instant::now();
        let outcome = black_box(battle.run(turns));
        let secs = start.elapsed().as_secs_f64();
        let per_second = outcome.turns as f64 / secs.max(1e-9);
        println!(
            "{:<28} {:>10} {:>14.0}",
            case.name, outcome.turns, per_second
        );
    }
}
