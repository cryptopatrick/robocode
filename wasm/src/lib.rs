//! Runs `robocode` battles for the browser battle viewer.
//!
//! A battle is simulated to the end and returned as a JSON **recording**
//! (format `roboverus-recording`, version 2); see [`robocode::recording`].

use robocode::live::{LiveSession, Pacer};
use robocode::physics::BattleConfig;
use robocode::recording;
use robocode::roster;
use robocode::runner::Battle;
use robocode::terrain::Terrain;
use robocode::{Battlefield, BulletMode};
use wasm_bindgen::prelude::*;

/// Robots that can be picked in the browser.
pub const ROBOT_NAMES: [&str; roster::BUILT_IN.len()] = roster::BUILT_IN;

/// What to battle.
#[derive(Debug, Clone, PartialEq)]
pub struct BattleSpec {
    pub robots: Vec<String>,
    pub width: f64,
    pub height: f64,
    pub max_turns: u64,
    /// The fixed urban map (buildings and roads) instead of an empty arena.
    pub urban: bool,
    /// Tank id per robot; empty means none picked (decoration only).
    pub tanks: Vec<String>,
    /// How fast bullets fly; recorded in the header.
    pub bullets: BulletMode,
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
        .map(|name| {
            roster::built_in(name)
                .ok_or_else(|| format!("unknown robot: {name}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    recording::validate_setup(n, spec.width, spec.height, spec.max_turns)?;
    let config = BattleConfig {
        battlefield: Battlefield::new(spec.width, spec.height),
        bullet_mode: spec.bullets,
        ..BattleConfig::default()
    };
    let mut battle = Battle::new(
        config,
        &roster::spawns(n, spec.width, spec.height),
        robots,
    );
    if spec.urban {
        battle.set_terrain(Terrain::urban(spec.width, spec.height));
    }
    battle.set_tanks(spec.tanks.clone());
    Ok(battle)
}

/// Runs the battle described by `spec` and returns its JSON recording.
pub fn record_battle(spec: &BattleSpec) -> Result<String, String> {
    Ok(recording::record(build_battle(spec)?, spec.max_turns))
}

fn spec_from_csv(
    robots_csv: &str,
    width: f64,
    height: f64,
    max_turns: u32,
    urban: bool,
    tanks_csv: &str,
    bullets: &str,
) -> Result<BattleSpec, String> {
    let list = |csv: &str| -> Vec<String> {
        csv.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    // Empty means the default (realistic).
    let bullets = match bullets.trim() {
        "" => BulletMode::default(),
        b => b.parse::<BulletMode>()?,
    };
    Ok(BattleSpec {
        robots: list(robots_csv),
        tanks: list(tanks_csv),
        width,
        height,
        max_turns: max_turns as u64,
        urban,
        bullets,
    })
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
    pub fn new(
        robots_csv: &str,
        width: f64,
        height: f64,
        max_turns: u32,
        urban: bool,
        tanks_csv: &str,
        bullets: &str,
    ) -> Result<LiveBattle, JsError> {
        Self::create(
            robots_csv, width, height, max_turns, urban, tanks_csv, bullets,
        )
        .map_err(|e| JsError::new(&e))
    }

    /// The v2 header object (robots, battlefield, bullets, tanks, terrain).
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
        let frames: Vec<String> = (0..n)
            .map_while(|_| self.session.step())
            .map(|f| f.to_json())
            .collect();
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
    pub fn create(
        robots_csv: &str,
        width: f64,
        height: f64,
        max_turns: u32,
        urban: bool,
        tanks_csv: &str,
        bullets: &str,
    ) -> Result<LiveBattle, String> {
        let spec = spec_from_csv(
            robots_csv, width, height, max_turns, urban, tanks_csv, bullets,
        )?;
        Ok(LiveBattle {
            session: LiveSession::new(build_battle(&spec)?, spec.max_turns),
        })
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
    let quoted: Vec<String> =
        ROBOT_NAMES.iter().map(|n| format!("\"{n}\"")).collect();
    format!("[{}]", quoted.join(","))
}

/// Native twin of [`run_battle`] (errors as strings).
pub fn run_battle_native(
    robots_csv: &str,
    width: f64,
    height: f64,
    max_turns: u32,
    urban: bool,
    tanks_csv: &str,
    bullets: &str,
) -> Result<String, String> {
    record_battle(&spec_from_csv(
        robots_csv, width, height, max_turns, urban, tanks_csv, bullets,
    )?)
}

/// Runs a battle; `robots_csv` is a comma-separated list of robot names and
/// `tanks_csv` one of tank ids (empty for none). `bullets` is `"realistic"` or
/// `"arcade"`; empty means realistic.
/// Throws a JS `Error` with a readable message if the setup is invalid.
#[wasm_bindgen]
pub fn run_battle(
    robots_csv: &str,
    width: f64,
    height: f64,
    max_turns: u32,
    urban: bool,
    tanks_csv: &str,
    bullets: &str,
) -> Result<String, JsError> {
    run_battle_native(
        robots_csv, width, height, max_turns, urban, tanks_csv, bullets,
    )
    .map_err(|e| JsError::new(&e))
}

/// The `[width, height]` of a battlefield ratio ("4:3", "2:1", "16:9" or
/// "1:1"), so the page never keeps its own table of sizes.
pub fn ratio_size(name: &str) -> Result<Vec<f64>, String> {
    let (w, h) = name.parse::<robocode::Ratio>()?.size();
    Ok(vec![w, h])
}

/// JS entry point for [`ratio_size`]; throws a JS `Error` on an unknown ratio.
#[wasm_bindgen(js_name = ratio_size)]
pub fn ratio_size_js(name: &str) -> Result<Vec<f64>, JsError> {
    ratio_size(name).map_err(|e| JsError::new(&e))
}
