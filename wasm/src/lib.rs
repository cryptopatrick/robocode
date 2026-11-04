//! Runs `robocode` battles for the browser battle viewer.
//!
//! A battle is simulated to the end and returned as a JSON **recording**
//! (format `roboverus-recording`, version 1): every turn's snapshot plus what
//! each robot observed and decided. See `docs/viewer.md` for the format.

use std::fmt::Write as _;

use robocode::physics::{BattleConfig, SpawnSpec, TurnEvents};
use robocode::robots::{SafeShooter, SittingDuck, Spinner};
use robocode::runner::Battle;
use robocode::strategy::{Action, Observation, Strategy};
use robocode::Battlefield;
use wasm_bindgen::prelude::*;

/// Robots that can be picked in the browser.
pub const ROBOT_NAMES: [&str; 3] = ["SafeShooter", "Spinner", "SittingDuck"];

/// What to battle.
#[derive(Debug, Clone, PartialEq)]
pub struct BattleSpec {
    pub robots: Vec<String>,
    pub width: f64,
    pub height: f64,
    pub max_turns: u64,
}

fn strategy(name: &str) -> Option<Box<dyn Strategy>> {
    match name {
        "SafeShooter" => Some(Box::new(SafeShooter)),
        "Spinner" => Some(Box::new(Spinner)),
        "SittingDuck" => Some(Box::new(SittingDuck)),
        _ => None,
    }
}

fn opt<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map_or_else(|| "null".to_string(), |v| v.to_string())
}

fn decision_json((obs, action): &(Observation, Action)) -> String {
    let target = obs.target.map_or_else(
        || "null".to_string(),
        |t| format!(r#"{{"bearing":{},"distance":{},"age":{}}}"#, t.bearing.0, t.distance, t.age),
    );
    format!(
        r#"{{"energy":{},"gun_heat":{},"gun_heading":{},"target":{},"fire":{},"ahead":{},"turn_body":{},"turn_gun":{},"turn_radar":{}}}"#,
        obs.energy.0,
        obs.gun_heat.0,
        obs.gun_heading.0,
        target,
        opt(action.fire.map(|p| p.0)),
        opt(action.ahead),
        opt(action.turn_body),
        opt(action.turn_gun),
        opt(action.turn_radar),
    )
}

/// Runs the battle described by `spec` and returns its JSON recording.
pub fn record_battle(spec: &BattleSpec) -> Result<String, String> {
    let n = spec.robots.len();
    if !(2..=4).contains(&n) {
        return Err(format!("need 2 to 4 robots, got {n}"));
    }
    let strategies = spec
        .robots
        .iter()
        .map(|name| strategy(name).ok_or_else(|| format!("unknown robot: {name}")))
        .collect::<Result<Vec<_>, _>>()?;
    let side_ok = |s: f64| (400.0..=5000.0).contains(&s);
    if !side_ok(spec.width) || !side_ok(spec.height) {
        return Err("arena must be 400..=5000 px per side".into());
    }
    if !(1..=20_000).contains(&spec.max_turns) {
        return Err("max_turns must be 1..=20000".into());
    }

    let (w, h) = (spec.width, spec.height);
    let corners = [(100.0, 100.0), (w - 100.0, h - 100.0), (w - 100.0, 100.0), (100.0, h - 100.0)];
    let spawns: Vec<SpawnSpec> = (0..n).map(|i| SpawnSpec::new(i, corners[i].0, corners[i].1, 0.0)).collect();
    let config = BattleConfig { battlefield: Battlefield::new(w, h), ..BattleConfig::default() };
    let mut battle = Battle::new(config, &spawns, strategies);

    let mut turns = vec![battle.state().snapshot("viewer", &TurnEvents::default()).to_json_string()];
    let mut decisions = vec![vec!["null".to_string(); n]];
    let outcome = loop {
        let alive = battle.state().robots.iter().filter(|r| r.alive).count();
        if alive <= 1 || turns.len() as u64 > spec.max_turns {
            break battle.run(0);
        }
        let events = battle.step();
        turns.push(battle.state().snapshot("viewer", &events).to_json_string());
        decisions.push((0..n).map(|i| battle.last_decision(i).as_ref().map_or_else(|| "null".into(), decision_json)).collect());
    };

    let mut out = String::from(r#"{"format":"roboverus-recording","version":1,"robots":["#);
    for (i, name) in spec.robots.iter().enumerate() {
        let sep = if i > 0 { "," } else { "" };
        let _ = write!(out, r#"{sep}{{"id":{i},"name":"{name}"}}"#);
    }
    let _ = write!(out, r#"],"battlefield":{{"width":{w},"height":{h}}},"turns":[{}],"decisions":["#, turns.join(","));
    let rows: Vec<String> = decisions.iter().map(|row| format!("[{}]", row.join(","))).collect();
    out.push_str(&rows.join(","));
    let energies: Vec<String> = outcome.energies.iter().map(|e| e.to_string()).collect();
    let _ = write!(
        out,
        r#"],"outcome":{{"winner":{},"turns":{},"energies":[{}]}}}}"#,
        opt(outcome.winner),
        turns.len() - 1,
        energies.join(",")
    );
    Ok(out)
}

/// JSON array of the robot names that can be picked.
#[wasm_bindgen]
pub fn robot_names() -> String {
    let quoted: Vec<String> = ROBOT_NAMES.iter().map(|n| format!("\"{n}\"")).collect();
    format!("[{}]", quoted.join(","))
}

/// Runs a battle; `robots_csv` is a comma-separated list of robot names.
/// Throws a JS `Error` with a readable message if the setup is invalid.
#[wasm_bindgen]
pub fn run_battle(robots_csv: &str, width: f64, height: f64, max_turns: u32) -> Result<String, JsError> {
    let spec = BattleSpec {
        robots: robots_csv.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
        width,
        height,
        max_turns: max_turns as u64,
    };
    record_battle(&spec).map_err(|e| JsError::new(&e))
}
