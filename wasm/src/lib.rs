//! Runs `robocode` battles for the browser battle viewer.
//!
//! A battle is simulated to the end and returned as a JSON **recording**
//! (format `roboverus-recording`, version 1); see [`robocode::recording`].

use robocode::Battlefield;
use robocode::live::{LiveSession, Pacer};
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

/// Builds the battle described by `spec`, checking it like the viewer does.
fn build_battle(spec: &BattleSpec) -> Result<Battle, String> {
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
    Ok(Battle::new(config, &roster::spawns(n, spec.width, spec.height), robots))
}

/// Runs the battle described by `spec` and returns its JSON recording.
pub fn record_battle(spec: &BattleSpec) -> Result<String, String> {
    Ok(recording::record(build_battle(spec)?, spec.max_turns))
}

fn spec_from_csv(robots_csv: &str, width: f64, height: f64, max_turns: u32) -> BattleSpec {
    BattleSpec {
        robots: robots_csv.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
        width,
        height,
        max_turns: max_turns as u64,
    }
}

/// A battle advanced by the page's (or a worker's) own clock, a turn at a time.
#[wasm_bindgen]
pub struct LiveBattle {
    session: LiveSession,
}

#[wasm_bindgen]
impl LiveBattle {
    /// Same robots and checks as [`run_battle`]; throws on a bad setup.
    #[wasm_bindgen(constructor)]
    pub fn new(robots_csv: &str, width: f64, height: f64, max_turns: u32) -> Result<LiveBattle, JsError> {
        Self::create(robots_csv, width, height, max_turns).map_err(|e| JsError::new(&e))
    }

    /// The v1 header object.
    pub fn header(&self) -> String {
        self.session.header().to_string()
    }

    /// The turn-0 frame (`{"turn":…,"decisions":[…]}`).
    pub fn initial_frame(&self) -> String {
        self.session.initial_frame().to_json()
    }

    /// Advances up to `n` turns; returns a JSON array of their frames (shorter
    /// if the battle finishes).
    pub fn step_many(&mut self, n: u32) -> String {
        let frames: Vec<String> = (0..n).map_while(|_| self.session.step()).map(|f| f.to_json()).collect();
        format!("[{}]", frames.join(","))
    }

    pub fn finished(&self) -> bool {
        self.session.finished()
    }

    /// The recording so far (complete once finished).
    pub fn recording(&self) -> String {
        self.session.recording()
    }
}

impl LiveBattle {
    /// Native constructor (errors as strings).
    pub fn create(robots_csv: &str, width: f64, height: f64, max_turns: u32) -> Result<LiveBattle, String> {
        let spec = spec_from_csv(robots_csv, width, height, max_turns);
        Ok(LiveBattle { session: LiveSession::new(build_battle(&spec)?, spec.max_turns) })
    }
}

/// Turn pacing for the page's live loop ([`robocode::live::Pacer`]), in milliseconds.
#[wasm_bindgen]
pub struct LivePacer {
    pacer: Pacer,
}

fn ns(ms: f64) -> u64 {
    (ms.max(0.0) * 1_000_000.0) as u64
}

#[wasm_bindgen]
impl LivePacer {
    /// `tps` turns per second; 0 means as fast as possible.
    #[wasm_bindgen(constructor)]
    pub fn new(tps: u32) -> LivePacer {
        LivePacer { pacer: Pacer::new((tps > 0).then_some(tps)) }
    }
    /// Turns due at `now_ms` (e.g. `performance.now()`).
    pub fn due(&mut self, now_ms: f64) -> u32 {
        self.pacer.due(ns(now_ms))
    }
    pub fn pause(&mut self) {
        self.pacer.pause();
    }
    pub fn resume(&mut self, now_ms: f64) {
        self.pacer.resume(ns(now_ms));
    }
    /// One turn while paused.
    pub fn step(&mut self) {
        self.pacer.request_step();
    }
    /// 0 means as fast as possible.
    pub fn set_tps(&mut self, tps: u32, now_ms: f64) {
        self.pacer.set_tps((tps > 0).then_some(tps), ns(now_ms));
    }
    pub fn measured_tps(&self) -> f64 {
        self.pacer.measured_tps()
    }
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
    record_battle(&spec_from_csv(robots_csv, width, height, max_turns)).map_err(|e| JsError::new(&e))
}
