//! Reads a `roboverus-recording` (version 1 or 2) into typed values for the export.

use serde::Deserialize;

/// A whole recording, as the export needs it.
#[derive(Debug, Clone, PartialEq)]
pub struct Replay {
    pub width: f64,
    pub height: f64,
    /// Sorted by id.
    pub robots: Vec<RobotInfo>,
    /// `"arcade"` when the recording has no `bullets` field.
    pub bullet_mode: String,
    pub tanks: Option<Vec<String>>,
    /// The winner's name; `None` when there is none.
    pub winner: Option<String>,
    /// `Map::default()` when the recording has no terrain.
    pub map: Map,
    /// Index = turn; 0 is the spawn state.
    pub turns: Vec<Turn>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RobotInfo {
    pub id: usize,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Turn {
    pub robots: Vec<RobotFrame>,
    pub bullets: Vec<BulletFrame>,
    pub events: Events,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RobotFrame {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub heading: f64,
    pub gun_heading: f64,
    pub radar_heading: f64,
    pub alive: bool,
    pub radius: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BulletFrame {
    pub id: u64,
    pub x: f64,
    pub y: f64,
    pub alive: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Events {
    pub fired: Vec<Fired>,
    pub hits: Vec<Hit>,
    pub obstacle_hits: Vec<ObstacleHit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fired {
    pub shooter: usize,
    pub bullet_id: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub victim: usize,
    pub bullet_id: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObstacleHit {
    pub bullet_id: u64,
    pub x: f64,
    pub y: f64,
}

/// The arena's terrain, including the optional real-city fields (see `docs/viewer.md`).
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default)]
pub struct Map {
    pub px_per_m: Option<f64>,
    pub buildings: Vec<[f64; 4]>,
    pub heights: Vec<f64>,
    pub shapes: Vec<Shape2d>,
    pub roads: Vec<[f64; 4]>,
    pub road_lines: Vec<RoadLine>,
    pub crossings: Vec<Crossing>,
    pub signals: Vec<Signal>,
    pub entrances: Vec<Entrance>,
    pub labels: Vec<Label>,
    pub name: Option<String>,
    pub attribution: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Shape2d {
    pub ring: Vec<[f64; 2]>,
    pub h: f64,
    #[serde(default)]
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RoadLine {
    #[serde(default)]
    pub class: String,
    pub width: f64,
    pub points: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Crossing {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    pub len: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Signal {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Entrance {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Label {
    pub text: String,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub h: f64,
}

#[derive(Deserialize)]
struct RawRecording {
    format: String,
    version: u32,
    robots: Vec<RawRobotInfo>,
    battlefield: RawBattlefield,
    #[serde(default)]
    bullets: Option<String>,
    #[serde(default)]
    tanks: Option<Vec<String>>,
    #[serde(default)]
    terrain: Option<Map>,
    turns: Vec<RawTurn>,
    #[serde(default)]
    outcome: Option<RawOutcome>,
}

#[derive(Deserialize)]
struct RawRobotInfo {
    id: usize,
    name: String,
}

#[derive(Deserialize)]
struct RawBattlefield {
    width: f64,
    height: f64,
}

#[derive(Deserialize)]
struct RawOutcome {
    #[serde(default)]
    winner: Option<usize>,
}

#[derive(Deserialize, Clone, Copy)]
struct RawPoint {
    x: f64,
    y: f64,
}

#[derive(Deserialize)]
struct RawTurn {
    #[serde(default)]
    robots: Vec<RawRobot>,
    #[serde(default)]
    bullets: Vec<RawBullet>,
    #[serde(default)]
    events: RawEvents,
}

#[derive(Deserialize)]
struct RawRobot {
    id: usize,
    position: RawPoint,
    heading: f64,
    gun_heading: f64,
    radar_heading: f64,
    alive: bool,
    #[serde(default = "default_radius")]
    radius: f64,
}

fn default_radius() -> f64 {
    18.0
}

#[derive(Deserialize)]
struct RawBullet {
    id: u64,
    position: RawPoint,
    alive: bool,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawEvents {
    bullet_fired: Vec<RawFired>,
    bullet_hits: Vec<RawHit>,
    bullet_hit_obstacles: Vec<RawObstacleHit>,
}

#[derive(Deserialize)]
struct RawFired {
    shooter: usize,
    bullet_id: u64,
}

#[derive(Deserialize)]
struct RawHit {
    victim: usize,
    bullet_id: u64,
}

#[derive(Deserialize)]
struct RawObstacleHit {
    bullet_id: u64,
    position: RawPoint,
}

/// Parses a recording. Errors read "not a robocode recording: …" or "the recording has no turns".
pub fn parse(json: &str) -> Result<Replay, String> {
    let raw: RawRecording = serde_json::from_str(json)
        .map_err(|e| format!("not a robocode recording: {e}"))?;
    if raw.format != "roboverus-recording" {
        return Err(format!(
            "not a robocode recording: format is {:?}, expected \"roboverus-recording\"",
            raw.format
        ));
    }
    if !(1..=2).contains(&raw.version) {
        return Err(format!(
            "not a robocode recording: version {} is not 1 or 2",
            raw.version
        ));
    }
    if raw.turns.is_empty() {
        return Err("the recording has no turns".into());
    }
    let mut robots: Vec<RobotInfo> = raw
        .robots
        .into_iter()
        .map(|r| RobotInfo { id: r.id, name: r.name })
        .collect();
    robots.sort_by_key(|r| r.id);
    let winner = raw
        .outcome
        .and_then(|o| o.winner)
        .and_then(|w| robots.iter().find(|r| r.id == w))
        .map(|r| r.name.clone());
    let turns = raw.turns.into_iter().map(turn).collect();
    Ok(Replay {
        width: raw.battlefield.width,
        height: raw.battlefield.height,
        robots,
        bullet_mode: raw.bullets.unwrap_or_else(|| "arcade".into()),
        tanks: raw.tanks,
        winner,
        map: raw.terrain.unwrap_or_default(),
        turns,
    })
}

fn turn(t: RawTurn) -> Turn {
    let mut robots: Vec<RobotFrame> = t
        .robots
        .into_iter()
        .map(|r| RobotFrame {
            id: r.id,
            x: r.position.x,
            y: r.position.y,
            heading: r.heading,
            gun_heading: r.gun_heading,
            radar_heading: r.radar_heading,
            alive: r.alive,
            radius: r.radius,
        })
        .collect();
    robots.sort_by_key(|r| r.id);
    let mut bullets: Vec<BulletFrame> = t
        .bullets
        .into_iter()
        .map(|b| BulletFrame {
            id: b.id,
            x: b.position.x,
            y: b.position.y,
            alive: b.alive,
        })
        .collect();
    bullets.sort_by_key(|b| b.id);
    let e = t.events;
    Turn {
        robots,
        bullets,
        events: Events {
            fired: e
                .bullet_fired
                .into_iter()
                .map(|f| Fired { shooter: f.shooter, bullet_id: f.bullet_id })
                .collect(),
            hits: e
                .bullet_hits
                .into_iter()
                .map(|h| Hit { victim: h.victim, bullet_id: h.bullet_id })
                .collect(),
            obstacle_hits: e
                .bullet_hit_obstacles
                .into_iter()
                .map(|o| ObstacleHit {
                    bullet_id: o.bullet_id,
                    x: o.position.x,
                    y: o.position.y,
                })
                .collect(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1: &str =
        include_str!("../../../tests/fixtures/open_arena_0_5_0.json");
    const GINZA: &str =
        include_str!("../../tests/fixtures/ginzabattle02.json");

    fn tiny(extra_header: &str, events: &str, winner: &str) -> String {
        format!(
            r#"{{"format":"roboverus-recording","version":2,
            "robots":[{{"id":1,"name":"B"}},{{"id":0,"name":"A"}}],
            "battlefield":{{"width":800,"height":600}}{extra_header},
            "turns":[
              {{"turn":0,"robots":[{{"id":0,"position":{{"x":100,"y":100}},"heading":0,"gun_heading":0,"radar_heading":0,"alive":true,"radius":18}}],"bullets":[]}},
              {{"turn":1,"robots":[],"bullets":[{{"id":3,"owner_id":0,"position":{{"x":5,"y":6}},"heading":0,"power":1,"speed":300,"alive":true}}],"events":{events}}}
            ],
            "outcome":{{"winner":{winner},"turns":1}}}}"#
        )
    }

    #[test]
    fn parses_version_1_fixture() {
        let r = parse(V1).unwrap();
        assert_eq!(r.bullet_mode, "arcade");
        assert_eq!(r.map, Map::default());
        assert_eq!(r.tanks, None);
        assert_eq!(r.width, 800.0);
        assert!(r.turns.len() > 1);
        let r0 = &r.turns[0].robots[0];
        assert_eq!((r0.id, r0.x, r0.y, r0.heading), (0, 100.0, 100.0, 0.0));
        assert_eq!(r.robots[0].name, "SafeShooter");
    }

    #[test]
    fn parses_ginza() {
        let r = parse(GINZA).unwrap();
        assert_eq!(r.turns.len(), 2001);
        assert_eq!((r.width, r.height), (1200.0, 600.0));
        let names: Vec<_> = r.robots.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            ["Candidate", "Circler", "Tactician", "SittingDuck"]
        );
        assert_eq!(r.winner, None);
        assert_eq!(r.bullet_mode, "realistic");
        assert_eq!(r.tanks.as_ref().unwrap().len(), 4);
        let m = &r.map;
        assert_eq!(m.px_per_m, Some(4.0));
        assert_eq!(
            (
                m.buildings.len(),
                m.heights.len(),
                m.shapes.len(),
                m.road_lines.len()
            ),
            (45, 45, 77, 198)
        );
        assert_eq!(
            (
                m.crossings.len(),
                m.signals.len(),
                m.entrances.len(),
                m.labels.len()
            ),
            (39, 8, 16, 27)
        );
        assert_eq!(m.name.as_deref(), Some("Ginza, Tokyo"));
        assert!(m.attribution.as_deref().unwrap().contains("OpenStreetMap"));
        let fired: usize = r.turns.iter().map(|t| t.events.fired.len()).sum();
        assert_eq!(fired, 40);
        let r3 = r.turns[65].robots.iter().find(|f| f.id == 3).unwrap();
        assert!(!r3.alive);
        assert!(r.turns[64].robots.iter().find(|f| f.id == 3).unwrap().alive);
    }

    #[test]
    fn parses_header_and_events() {
        let json = tiny(
            r#","bullets":"realistic","tanks":["k2","abrams"],"terrain":{"buildings":[[10,20,30,40]],"roads":[[0,0,800,20]]}"#,
            r#"{"bullet_fired":[{"shooter":0,"bullet_id":3,"power":1}],"bullet_hits":[{"shooter":0,"victim":1,"bullet_id":3,"power":1}],"bullet_hit_obstacles":[{"bullet_id":4,"owner":0,"power":1,"position":{"x":10,"y":20}}],"robot_deaths":[1]}"#,
            "1",
        );
        let r = parse(&json).unwrap();
        assert_eq!(r.robots[0], RobotInfo { id: 0, name: "A".into() });
        assert_eq!(r.winner.as_deref(), Some("B"));
        assert_eq!(r.bullet_mode, "realistic");
        assert_eq!(r.tanks, Some(vec!["k2".into(), "abrams".into()]));
        assert_eq!(r.map.buildings, vec![[10.0, 20.0, 30.0, 40.0]]);
        assert_eq!(r.map.roads, vec![[0.0, 0.0, 800.0, 20.0]]);
        let e = &r.turns[1].events;
        assert_eq!(e.fired, vec![Fired { shooter: 0, bullet_id: 3 }]);
        assert_eq!(e.hits, vec![Hit { victim: 1, bullet_id: 3 }]);
        assert_eq!(
            e.obstacle_hits,
            vec![ObstacleHit { bullet_id: 4, x: 10.0, y: 20.0 }]
        );
        assert_eq!(
            r.turns[1].bullets,
            vec![BulletFrame { id: 3, x: 5.0, y: 6.0, alive: true }]
        );
        assert_eq!(r.turns[0].events, Events::default());
    }

    #[test]
    fn winner_null_is_none() {
        let r = parse(&tiny("", "{}", "null")).unwrap();
        assert_eq!(r.winner, None);
    }

    #[test]
    fn rejects_non_json() {
        assert!(
            parse("not json")
                .unwrap_err()
                .contains("not a robocode recording")
        );
    }

    #[test]
    fn rejects_wrong_format() {
        let json =
            tiny("", "{}", "null").replace("roboverus-recording", "other");
        assert!(
            parse(&json).unwrap_err().contains("not a robocode recording")
        );
        assert!(parse("{}").unwrap_err().contains("not a robocode recording"));
    }

    #[test]
    fn rejects_zero_turns() {
        let json = r#"{"format":"roboverus-recording","version":2,"robots":[],"battlefield":{"width":800,"height":600},"turns":[]}"#;
        assert!(parse(json).unwrap_err().contains("no turns"));
    }
}
