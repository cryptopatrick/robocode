//! SafeShooter against a sample robot, headless.
//!
//! ```sh
//! cargo run --example battle -- spinner        # or: sitting_duck
//! ```

use robocode::physics::{BattleConfig, SpawnSpec};
use robocode::robots::{SafeShooter, SittingDuck, Spinner};
use robocode::runner::Battle;
use robocode::strategy::Strategy;

fn main() {
    let opponent_name = std::env::args().nth(1).unwrap_or_else(|| "sitting_duck".into());
    let opponent: Box<dyn Strategy> = match opponent_name.as_str() {
        "sitting_duck" => Box::new(SittingDuck),
        "spinner" => Box::new(Spinner),
        other => {
            eprintln!("unknown opponent `{other}`; use sitting_duck or spinner");
            std::process::exit(2);
        }
    };
    let spawns = [SpawnSpec::new(0, 100.0, 100.0, 0.0), SpawnSpec::new(1, 700.0, 500.0, 0.0)];
    let mut battle = Battle::new(BattleConfig::default(), &spawns, vec![Box::new(SafeShooter), opponent]);
    let outcome = battle.run(5000);
    let winner = outcome.winner.map_or("none", |i| battle.name(i));
    println!(
        "winner={winner} turns={} energy={:.1}/{:.1}",
        outcome.turns, outcome.energies[0], outcome.energies[1]
    );
}
