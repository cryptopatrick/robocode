use robocode_wasm::{BattleSpec, record_battle};
use serde_json::Value;

fn spec(robots: &[&str]) -> BattleSpec {
    BattleSpec { robots: robots.iter().map(|s| s.to_string()).collect(), width: 800.0, height: 600.0, max_turns: 5000, urban: false }
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

#[test]
fn record_with_circler() {
    let r = record(&["Circler", "SittingDuck"]);
    assert_eq!(r["robots"][0]["name"], "Circler");
    let decisions = r["decisions"].as_array().unwrap();
    // Circler is an API robot, not a fixed-point strategy: no decisions recorded.
    assert!(decisions.iter().all(|row| row[0].is_null()));
    assert!(!decisions[1][1].is_null());
}

/// FNV-1a, to fingerprint a recording without storing it.
fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3))
}

#[test]
fn recording_fingerprint_is_unchanged() {
    let json = record_battle(&spec(&["SafeShooter", "Spinner"])).unwrap();
    assert_eq!((json.len(), fnv1a(&json)), (716_414, 0xa87d_8e29_504d_8bfd));
}

#[test]
fn live_battle_streams_the_same_battle() {
    use robocode_wasm::LiveBattle;
    let expected: Value = serde_json::from_str(&record_battle(&spec(&["SafeShooter", "Spinner"])).unwrap()).unwrap();
    let mut live = LiveBattle::create("SafeShooter,Spinner", 800.0, 600.0, 5000, false).unwrap();
    let header: Value = serde_json::from_str(&live.header()).unwrap();
    assert_eq!(header["robots"], expected["robots"]);
    let mut turns = vec![serde_json::from_str::<Value>(&live.initial_frame()).unwrap()["turn"].clone()];
    while !live.finished() {
        let batch: Value = serde_json::from_str(&live.step_many(7)).unwrap();
        turns.extend(batch.as_array().unwrap().iter().map(|f| f["turn"].clone()));
    }
    assert_eq!(Value::Array(turns), expected["turns"]);
    let finished: Value = serde_json::from_str(&live.recording()).unwrap();
    assert_eq!(finished, expected);
}

#[test]
fn live_pacer_counts_due_turns_from_milliseconds() {
    use robocode_wasm::LivePacer;
    let mut p = LivePacer::new(30);
    let total: u32 = (0..1000).map(|ms| p.due(ms as f64)).sum();
    assert_eq!(total, 30);
    p.pause();
    assert_eq!(p.due(1500.0), 0);
    p.step();
    assert_eq!(p.due(1501.0), 1);
}

#[test]
fn an_urban_recording_carries_its_terrain_and_an_empty_one_does_not() {
    let urban = BattleSpec { urban: true, ..spec(&["SafeShooter", "Spinner"]) };
    let json: Value = serde_json::from_str(&record_battle(&urban).unwrap()).unwrap();
    assert_eq!(json["terrain"]["buildings"].as_array().unwrap().len(), 9);
    assert_eq!(json["terrain"]["roads"].as_array().unwrap().len(), 2);

    let plain: Value = serde_json::from_str(&record_battle(&spec(&["SafeShooter", "Spinner"])).unwrap()).unwrap();
    assert!(plain.get("terrain").is_none());
}

#[test]
fn bullets_are_absorbed_in_an_urban_battle() {
    let urban = BattleSpec { urban: true, ..spec(&["Spinner", "SafeShooter"]) };
    let json: Value = serde_json::from_str(&record_battle(&urban).unwrap()).unwrap();
    let absorbed: usize = json["turns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["events"]["bullet_hit_obstacles"].as_array().map_or(0, Vec::len))
        .sum();
    assert!(absorbed > 0, "no bullet met a building in the whole battle");
}
