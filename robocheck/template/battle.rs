//! TRUSTED TEMPLATE (robocheck): the candidate fights a built-in robot.
//! Usage: battle --opponent NAME --turns N --arena open|urban --out PATH --decisions PATH
//! Prints one JSON line: {"artifact_hash": sha256 of the strategy.rs compiled
//! into this binary, "winner": name or null, "turns": N}.
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use vstd::prelude::*;

use robocheck_arena::ArenaRobot;
use robocode::Battlefield;
use robocode::physics::BattleConfig;
use robocode::runner::Battle;
use robocode::terrain::Terrain;
use robocode::{recording, roster};
use sha2::{Digest, Sha256};
use {{LIB}}::strategy::decide;

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
    let decisions = arg(&args, "--decisions").unwrap_or_else(|| "decisions.jsonl".into());
    let urban = match arg(&args, "--arena").as_deref() {
        None | Some("open") => false,
        Some("urban") => true,
        Some(other) => {
            eprintln!("unknown arena {other}; choose open or urban");
            std::process::exit(2);
        }
    };
    let Some(enemy) = roster::built_in(&opponent) else {
        eprintln!("unknown opponent {opponent}; choose one of {}", roster::BUILT_IN.join(", "));
        std::process::exit(2);
    };
    let (w, h) = (800.0, 600.0);
    // Realistic bullets, robocode's default; the recording header says so.
    let config = BattleConfig {
        battlefield: Battlefield::new(w, h),
        bullet_mode: robocode::BulletMode::Realistic,
        ..BattleConfig::default()
    };
    let terrain = if urban { Terrain::urban(w, h) } else { Terrain::default() };
    let candidate = ArenaRobot::new("Candidate", decide, terrain.clone(), w, h);
    let log = candidate.log();
    let mut battle = Battle::new(config, &roster::spawns(2, w, h), vec![Box::new(candidate), enemy]);
    if urban {
        battle.set_terrain(terrain);
    }
    let rec = recording::record_with_summary(battle, turns);
    let lines: String = log.borrow().iter().map(|d| d.to_json_line() + "\n").collect();
    if let Err(e) = std::fs::write(&decisions, lines) {
        eprintln!("cannot write {decisions}: {e}");
        std::process::exit(2);
    }
    if let Err(e) = std::fs::write(&out, &rec.json) {
        eprintln!("cannot write {out}: {e}");
        std::process::exit(2);
    }
    let hash: String = Sha256::digest(STRATEGY_SRC.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    let winner = rec.winner.map_or_else(|| "null".to_string(), |n| format!("\"{n}\""));
    println!(r#"{{"artifact_hash":"{hash}","winner":{winner},"turns":{}}}"#, rec.turns);
}
