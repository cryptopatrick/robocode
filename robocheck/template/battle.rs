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

fn die(msg: String) -> ! {
    eprintln!("{msg}");
    std::process::exit(2);
}

/// `--buildings "x,y,w,h;x,y,w,h"`: the buildings of a custom map, in pixels.
fn parse_buildings(text: &str) -> Vec<robocode::Rect> {
    text.split(';')
        .filter(|r| !r.trim().is_empty())
        .map(|r| {
            let v: Vec<f64> = r.split(',').map(|n| n.trim().parse().unwrap_or(f64::NAN)).collect();
            if v.len() != 4 || v.iter().any(|n| !n.is_finite()) || v[2] <= 0.0 || v[3] <= 0.0 {
                die(format!("bad building {r:?}: want x,y,width,height"));
            }
            robocode::Rect::new(v[0], v[1], v[2], v[3])
        })
        .collect()
}

/// `--spawns "x,y;x,y;..."`: where each robot starts (candidate first), for maps whose corners are built over.
fn parse_spawns(text: &str, robots: usize, w: f64, h: f64) -> Vec<robocode::physics::SpawnSpec> {
    let pts: Vec<(f64, f64)> = text
        .split(';')
        .filter(|r| !r.trim().is_empty())
        .map(|r| {
            let v: Vec<f64> = r.split(',').map(|n| n.trim().parse().unwrap_or(f64::NAN)).collect();
            if v.len() != 2 || v.iter().any(|n| !n.is_finite()) || v[0] < 0.0 || v[0] > w || v[1] < 0.0 || v[1] > h {
                die(format!("bad spawn {r:?}: want x,y inside the arena"));
            }
            (v[0], v[1])
        })
        .collect();
    if pts.len() != robots {
        die(format!("--spawns gives {} points but {robots} robots fight", pts.len()));
    }
    pts.iter().enumerate().map(|(i, &(x, y))| robocode::physics::SpawnSpec::new(i, x, y, 0.0)).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // One opponent by default; `--opponents A,B,C` (up to three) fights them all at once.
    let opponents: Vec<String> = arg(&args, "--opponents")
        .or_else(|| arg(&args, "--opponent"))
        .unwrap_or_else(|| "SittingDuck".into())
        .split(',')
        .map(|n| n.trim().to_string())
        .collect();
    let turns: u64 = arg(&args, "--turns").and_then(|t| t.parse().ok()).unwrap_or(2000);
    let out = arg(&args, "--out").unwrap_or_else(|| "recording.json".into());
    let decisions = arg(&args, "--decisions").unwrap_or_else(|| "decisions.jsonl".into());
    let (w, h): (f64, f64) = (
        arg(&args, "--width").and_then(|v| v.parse().ok()).unwrap_or(800.0),
        arg(&args, "--height").and_then(|v| v.parse().ok()).unwrap_or(600.0),
    );
    if !(200.0..=3000.0).contains(&w) || !(200.0..=3000.0).contains(&h) {
        die(format!("arena {w} x {h} is outside 200 to 3000 px"));
    }
    let bullet_mode = match arg(&args, "--bullets").as_deref() {
        None | Some("realistic") => robocode::BulletMode::Realistic,
        Some("arcade") => robocode::BulletMode::Arcade,
        Some(other) => die(format!("unknown bullet mode {other}; choose realistic or arcade")),
    };
    // The map: the fixed city (`--arena urban`), your own buildings (`--buildings`), or none.
    let terrain = if let Some(text) = arg(&args, "--buildings") {
        Terrain { buildings: parse_buildings(&text), roads: Vec::new() }
    } else {
        match arg(&args, "--arena").as_deref() {
            None | Some("open") => Terrain::default(),
            Some("urban") => Terrain::urban(w, h),
            Some(other) => die(format!("unknown arena {other}; choose open or urban")),
        }
    };
    let has_terrain = !terrain.buildings.is_empty();
    if opponents.is_empty() || opponents.len() > 3 {
        die("choose one to three opponents".into());
    }
    let mut robots: Vec<Box<dyn robocode::api::Robot>> = Vec::new();
    let candidate = ArenaRobot::new("Candidate", decide, terrain.clone(), w, h);
    let log = candidate.log();
    robots.push(Box::new(candidate));
    for name in &opponents {
        match roster::built_in(name) {
            Some(r) => robots.push(r),
            None => die(format!("unknown opponent {name}; choose one of {}", roster::BUILT_IN.join(", "))),
        }
    }
    let config = BattleConfig {
        battlefield: Battlefield::new(w, h),
        bullet_mode,
        ..BattleConfig::default()
    };
    let spawns = match arg(&args, "--spawns") {
        Some(text) => parse_spawns(&text, robots.len(), w, h),
        None => roster::spawns(robots.len(), w, h),
    };
    let mut battle = Battle::new(config, &spawns, robots);
    if has_terrain {
        battle.set_terrain(terrain);
    }
    // Decoration only (physics never reads it). Default: the candidate a Leopard 2A8, then an Abrams, K2, Leclerc.
    let defaults = ["leopard2a8", "abrams", "k2", "leclerc", "merkava"];
    let tanks: Vec<String> = match arg(&args, "--tanks") {
        Some(t) => t.split(',').map(|n| n.trim().to_string()).collect(),
        None => defaults.iter().take(opponents.len() + 1).map(|n| n.to_string()).collect(),
    };
    if tanks.len() != opponents.len() + 1 || tanks.iter().any(|t| !defaults.contains(&t.as_str())) {
        die(format!("--tanks wants {} of {}", opponents.len() + 1, defaults.join(", ")));
    }
    battle.set_tanks(tanks);
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
