//! Battle recordings: format `roboverus-recording`, version 1.
//!
//! A recording holds every turn's snapshot plus what each fixed-point
//! strategy robot observed and decided. The battle viewer plays them back.
//! See `docs/viewer.md` for the format.

use std::fmt::Write as _;

use crate::physics::TurnEvents;
use crate::runner::Battle;
use crate::strategy::{Action, Observation};

/// Checks a battle setup, with the messages the viewer shows.
pub fn validate_setup(robots: usize, width: f64, height: f64, max_turns: u64) -> Result<(), String> {
    if !(2..=4).contains(&robots) {
        return Err(format!("need 2 to 4 robots, got {robots}"));
    }
    let side_ok = |s: f64| (400.0..=5000.0).contains(&s);
    if !side_ok(width) || !side_ok(height) {
        return Err("arena must be 400..=5000 px per side".into());
    }
    if !(1..=20_000).contains(&max_turns) {
        return Err("max_turns must be 1..=20000".into());
    }
    Ok(())
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

/// A finished battle: its JSON recording plus a summary.
#[derive(Debug, Clone, PartialEq)]
pub struct Recorded {
    pub json: String,
    /// The winner's name, if exactly one robot survived.
    pub winner: Option<String>,
    pub turns: u64,
}

impl Recorded {
    /// "Bot wins after 120 turns" or "No winner after 5000 turns".
    pub fn summary(&self) -> String {
        match &self.winner {
            Some(name) => format!("{name} wins after {} turns", self.turns),
            None => format!("No winner after {} turns", self.turns),
        }
    }
}

/// Runs `battle` until one robot is left or `max_turns` have run, and returns
/// its JSON recording.
pub fn record(battle: Battle, max_turns: u64) -> String {
    record_with_summary(battle, max_turns).json
}

/// Like [`record`], also returning who won and after how many turns.
pub fn record_with_summary(mut battle: Battle, max_turns: u64) -> Recorded {
    let n = battle.state().robots.len();
    let config = battle.state().config;
    let (w, h) = (config.battlefield.width, config.battlefield.height);

    let mut turns = vec![battle.state().snapshot("viewer", &TurnEvents::default()).to_json_string()];
    let mut decisions = vec![vec!["null".to_string(); n]];
    let outcome = loop {
        let alive = battle.state().robots.iter().filter(|r| r.alive).count();
        if alive <= 1 || turns.len() as u64 > max_turns {
            break battle.run(0);
        }
        let events = battle.step();
        turns.push(battle.state().snapshot("viewer", &events).to_json_string());
        decisions.push(
            (0..n).map(|i| battle.last_decision(i).as_ref().map_or_else(|| "null".into(), decision_json)).collect(),
        );
    };

    let mut out = String::from(r#"{"format":"roboverus-recording","version":1,"robots":["#);
    for i in 0..n {
        let sep = if i > 0 { "," } else { "" };
        let name = battle.name(i).replace('\\', "\\\\").replace('"', "\\\"");
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
    let winner = outcome.winner.and_then(|id| {
        battle.state().robots.iter().position(|r| r.id == id).map(|i| battle.name(i).to_string())
    });
    Recorded { json: out, winner, turns: (turns.len() - 1) as u64 }
}
