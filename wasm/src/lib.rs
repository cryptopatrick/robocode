//! Runs `robocode` battles for the browser battle viewer.
//!
//! A battle is simulated to the end and returned as a JSON **recording**
//! (format `roboverus-recording`, version 1); see [`robocode::recording`].

use robocode::Battlefield;
use robocode::physics::BattleConfig;
use robocode::recording;
use robocode::roster;
use robocode::runner::Battle;
use wasm_bindgen::prelude::*;

/// Robots that can be picked in the browser.
pub const ROBOT_NAMES: [&str; 4] = roster::BUILT_IN;

/// What to battle.
#[derive(Debug, Clone, PartialEq)]
pub struct BattleSpec {
    pub robots: Vec<String>,
    pub width: f64,
    pub height: f64,
    pub max_turns: u64,
}

/// Runs the battle described by `spec` and returns its JSON recording.
pub fn record_battle(spec: &BattleSpec) -> Result<String, String> {
    let n = spec.robots.len();
    if !(2..=4).contains(&n) {
        return Err(format!("need 2 to 4 robots, got {n}"));
    }
    let robots = spec
        .robots
        .iter()
        .map(|name| roster::built_in(name).ok_or_else(|| format!("unknown robot: {name}")))
        .collect::<Result<Vec<_>, _>>()?;
    recording::validate_setup(n, spec.width, spec.height, spec.max_turns)?;
    let config = BattleConfig { battlefield: Battlefield::new(spec.width, spec.height), ..BattleConfig::default() };
    let battle = Battle::new(config, &roster::spawns(n, spec.width, spec.height), robots);
    Ok(recording::record(battle, spec.max_turns))
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
