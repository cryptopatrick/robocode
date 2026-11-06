//! TRUSTED TEMPLATE (robocheck): the candidate fights a built-in robot.
//! Usage: battle --opponent NAME --turns N --out PATH
//! Prints one JSON line: {"artifact_hash": sha256 of the strategy.rs compiled
//! into this binary, "winner": name or null, "turns": N}.
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use vstd::prelude::*;

use robocode::Battlefield;
use robocode::physics::BattleConfig;
use robocode::runner::{Battle, boxed};
use robocode::{recording, roster};
use sha2::{Digest, Sha256};
use {{LIB}}::strategy::Candidate;

/// The exact source this binary was compiled from.
const STRATEGY_SRC: &str = include_str!("../strategy.rs");

fn arg(args: &[String], key: &str) -> Option<String> {
    args.iter().position(|a| a == key).and_then(|i| args.get(i + 1)).cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let opponent = arg(&args, "--opponent").unwrap_or_else(|| "SittingDuck".into());
    let turns: u64 = arg(&args, "--turns").and_then(|t| t.parse().ok()).unwrap_or(2000);
    let out = arg(&args, "--out").unwrap_or_else(|| "recording.json".into());
    let Some(enemy) = roster::built_in(&opponent) else {
        eprintln!("unknown opponent {opponent}; choose one of {}", roster::BUILT_IN.join(", "));
        std::process::exit(2);
    };
    let (w, h) = (800.0, 600.0);
    let config = BattleConfig { battlefield: Battlefield::new(w, h), ..BattleConfig::default() };
    let battle = Battle::new(config, &roster::spawns(2, w, h), vec![boxed(Candidate), enemy]);
    let rec = recording::record_with_summary(battle, turns);
    if let Err(e) = std::fs::write(&out, &rec.json) {
        eprintln!("cannot write {out}: {e}");
        std::process::exit(2);
    }
    let hash: String = Sha256::digest(STRATEGY_SRC.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    let winner = rec.winner.map_or_else(|| "null".to_string(), |n| format!("\"{n}\""));
    println!(r#"{{"artifact_hash":"{hash}","winner":{winner},"turns":{}}}"#, rec.turns);
}
