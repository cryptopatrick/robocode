use robocode_wasm::{BattleSpec, record_battle};
use serde_json::Value;

fn spec(robots: &[&str]) -> BattleSpec {
    BattleSpec { robots: robots.iter().map(|s| s.to_string()).collect(), width: 800.0, height: 600.0, max_turns: 5000 }
}

fn record(robots: &[&str]) -> Value {
    serde_json::from_str(&record_battle(&spec(robots)).expect("battle runs")).expect("valid JSON")
}

#[test]
fn records_safe_shooter_vs_spinner() {
    let r = record(&["SafeShooter", "Spinner"]);
    assert_eq!(r["format"], "roboverus-recording");
    assert_eq!(r["version"], 1);
    assert_eq!(r["robots"][1]["name"], "Spinner");
    assert_eq!(r["battlefield"]["width"], 800.0);
    let turns = r["turns"].as_array().unwrap();
    let decisions = r["decisions"].as_array().unwrap();
    let played = r["outcome"]["turns"].as_u64().unwrap() as usize;
    assert_eq!(turns.len(), played + 1);
    assert_eq!(decisions.len(), turns.len());
    assert!(decisions[0].as_array().unwrap().iter().all(Value::is_null));
    assert_eq!(r["outcome"]["winner"], 0);
    assert_eq!(turns[0]["turn"], 0);
    assert_eq!(turns[0]["robots"][0]["position"]["x"], 100.0);
    assert_eq!(turns[0]["robots"][0]["position"]["y"], 100.0);
    assert_eq!(turns[0]["robots"][1]["position"]["x"], 700.0);
    assert_eq!(turns[0]["robots"][1]["position"]["y"], 500.0);
    let d = &decisions[1][0];
    assert_eq!(d["energy"], 100_000);
    assert_eq!(d["turn_radar"], 450);
    assert!(d["target"].is_null());
}

#[test]
fn four_robots_spawn_in_corners() {
    let r = record(&["SittingDuck", "SittingDuck", "SittingDuck", "SafeShooter"]);
    let pos = |i: usize| (r["turns"][0]["robots"][i]["position"]["x"].as_f64().unwrap(), r["turns"][0]["robots"][i]["position"]["y"].as_f64().unwrap());
    assert_eq!(pos(2), (700.0, 100.0));
    assert_eq!(pos(3), (100.0, 500.0));
}

#[test]
fn dead_robot_decisions_are_null() {
    let r = record(&["SafeShooter", "SittingDuck"]);
    let turns = r["turns"].as_array().unwrap();
    let death = turns.iter().position(|t| t["robots"][1]["alive"] == false).expect("duck dies");
    let decisions = r["decisions"].as_array().unwrap();
    // The duck still decided on the turn it died; afterwards it never does.
    for (k, row) in decisions.iter().enumerate().skip(death + 1) {
        assert!(row[1].is_null(), "turn {k}");
    }
    assert!(!decisions[death][1].is_null());
}

#[test]
fn rejects_bad_specs() {
    let err = |s: BattleSpec| record_battle(&s).unwrap_err();
    assert_eq!(err(spec(&["SafeShooter"])), "need 2 to 4 robots, got 1");
    assert_eq!(err(spec(&["SafeShooter"; 5])), "need 2 to 4 robots, got 5");
    assert_eq!(err(spec(&["SafeShooter", "Nope"])), "unknown robot: Nope");
    assert_eq!(err(BattleSpec { width: 300.0, ..spec(&["SafeShooter", "Spinner"]) }), "arena must be 400..=5000 px per side");
    assert_eq!(err(BattleSpec { max_turns: 0, ..spec(&["SafeShooter", "Spinner"]) }), "max_turns must be 1..=20000");
}

#[test]
fn is_deterministic() {
    let s = spec(&["SafeShooter", "Spinner"]);
    assert_eq!(record_battle(&s), record_battle(&s));
}
