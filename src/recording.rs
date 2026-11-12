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

/// One recorded turn: its snapshot and every robot's decision (or `null`).
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// `TurnSnapshot` JSON.
    pub snapshot: String,
    /// JSON array, one decision (or `null`) per robot.
    pub decisions: String,
}

impl Frame {
    /// `{"turn":<snapshot>,"decisions":[…]}`, one line of a live stream.
    pub fn to_json(&self) -> String {
        format!(r#"{{"turn":{},"decisions":{}}}"#, self.snapshot, self.decisions)
    }
}

fn robots_and_field(battle: &Battle) -> String {
    let n = battle.state().robots.len();
    let field = battle.state().config.battlefield;
    let mut out = String::from(r#""robots":["#);
    for i in 0..n {
        let sep = if i > 0 { "," } else { "" };
        let name = battle.name(i).replace('\\', "\\\\").replace('"', "\\\"");
        let _ = write!(out, r#"{sep}{{"id":{i},"name":"{name}"}}"#);
    }
    let _ = write!(out, r#"],"battlefield":{{"width":{},"height":{}}}"#, field.width, field.height);
    let terrain = &battle.state().terrain;
    if !terrain.buildings.is_empty() || !terrain.roads.is_empty() {
        let rects = |rects: &[crate::geometry::Rect]| {
            rects
                .iter()
                .map(|r| format!("[{},{},{},{}]", r.min.x, r.min.y, r.max.x - r.min.x, r.max.y - r.min.y))
                .collect::<Vec<_>>()
                .join(",")
        };
        let _ = write!(out, r#","terrain":{{"buildings":[{}],"roads":[{}]}}"#, rects(&terrain.buildings), rects(&terrain.roads));
    }
    out
}

/// The recording's header as a standalone JSON object (first line of a live stream).
pub fn header_json(battle: &Battle) -> String {
    format!(r#"{{"format":"roboverus-recording","version":1,{}}}"#, robots_and_field(battle))
}

/// The frame for the battle's current state before any step (turn 0).
pub fn initial_frame(battle: &Battle) -> Frame {
    let n = battle.state().robots.len();
    Frame {
        snapshot: battle.state().snapshot("viewer", &TurnEvents::default()).to_json_string(),
        decisions: format!("[{}]", vec!["null"; n].join(",")),
    }
}

/// The frame for the turn `battle.step()` just produced (with its `events`).
pub fn frame(battle: &Battle, events: &TurnEvents) -> Frame {
    let n = battle.state().robots.len();
    let row: Vec<String> =
        (0..n).map(|i| battle.last_decision(i).as_ref().map_or_else(|| "null".into(), decision_json)).collect();
    Frame { snapshot: battle.state().snapshot("viewer", events).to_json_string(), decisions: format!("[{}]", row.join(",")) }
}

/// Builds a v1 recording one frame at a time (as a live battle produces them).
pub struct RecordingBuilder {
    head: String,
    turns: Vec<String>,
    decisions: Vec<String>,
}

impl RecordingBuilder {
    pub fn new(battle: &Battle) -> Self {
        RecordingBuilder { head: robots_and_field(battle), turns: Vec::new(), decisions: Vec::new() }
    }

    pub fn push(&mut self, frame: &Frame) {
        self.turns.push(frame.snapshot.clone());
        self.decisions.push(frame.decisions.clone());
    }

    /// Frames pushed so far.
    pub fn len(&self) -> usize {
        self.turns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.turns.is_empty()
    }

    /// The finished recording, with `outcome` (from `battle.run(0)` or similar).
    pub fn finish(&self, battle: &Battle, outcome: &crate::runner::BattleOutcome) -> Recorded {
        let played = self.turns.len().saturating_sub(1);
        let energies: Vec<String> = outcome.energies.iter().map(|e| e.to_string()).collect();
        let json = format!(
            r#"{{"format":"roboverus-recording","version":1,{},"turns":[{}],"decisions":[{}],"outcome":{{"winner":{},"turns":{},"energies":[{}]}}}}"#,
            self.head,
            self.turns.join(","),
            self.decisions.join(","),
            opt(outcome.winner),
            played,
            energies.join(",")
        );
        let winner = outcome.winner.and_then(|id| {
            battle.state().robots.iter().position(|r| r.id == id).map(|i| battle.name(i).to_string())
        });
        Recorded { json, winner, turns: played as u64 }
    }
}

/// Like [`record`], also returning who won and after how many turns.
pub fn record_with_summary(mut battle: Battle, max_turns: u64) -> Recorded {
    let mut builder = RecordingBuilder::new(&battle);
    builder.push(&initial_frame(&battle));
    let outcome = loop {
        let alive = battle.state().robots.iter().filter(|r| r.alive).count();
        if alive <= 1 || builder.len() as u64 > max_turns {
            break battle.run(0);
        }
        let events = battle.step();
        builder.push(&frame(&battle, &events));
    };
    builder.finish(&battle, &outcome)
}
