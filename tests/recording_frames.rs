use robocode::BulletMode;
use robocode::physics::BattleConfig;
use robocode::recording::{self, RecordingBuilder};
use robocode::roster;
use robocode::runner::Battle;

/// Pinned to arcade: these tests assume classic Robocode bullet speed.
fn arcade() -> BattleConfig {
    BattleConfig { bullet_mode: BulletMode::Arcade, ..BattleConfig::default() }
}

fn duel() -> Battle {
    let robots = vec![
        roster::built_in("SafeShooter").unwrap(),
        roster::built_in("Spinner").unwrap(),
    ];
    Battle::new(arcade(), &roster::spawns(2, 800.0, 600.0), robots)
}

#[test]
fn frames_rebuild_the_same_recording() {
    let expected = recording::record(duel(), 5000);

    let mut battle = duel();
    let mut builder = RecordingBuilder::new(&battle);
    builder.push(&recording::initial_frame(&battle));
    let mut turns = 0;
    while battle.state().robots.iter().filter(|r| r.alive).count() > 1
        && turns < 5000
    {
        let events = battle.step();
        builder.push(&recording::frame(&battle, &events));
        turns += 1;
    }
    let outcome = battle.run(0);
    assert_eq!(builder.finish(&battle, &outcome).json, expected);
}

#[test]
fn header_is_a_v2_object() {
    let h = recording::header_json(&duel());
    assert_eq!(
        h,
        r#"{"format":"roboverus-recording","version":2,"robots":[{"id":0,"name":"SafeShooter"},{"id":1,"name":"Spinner"}],"battlefield":{"width":800,"height":600},"bullets":"arcade"}"#
    );
}

#[test]
fn frame_json_holds_turn_and_decisions() {
    let mut battle = duel();
    let events = battle.step();
    let f = recording::frame(&battle, &events).to_json();
    assert!(
        f.starts_with(r#"{"turn":{"battle_id":"viewer","turn":1,"#),
        "{}",
        &f[..80]
    );
    assert!(
        f.contains(r#""decisions":[{"energy":100000"#),
        "{}",
        &f[f.len() - 300..]
    );
}

fn urban_battle(a: &str, b: &str) -> Battle {
    use robocode::terrain::Terrain;
    let robots =
        vec![roster::built_in(a).unwrap(), roster::built_in(b).unwrap()];
    let mut battle =
        Battle::new(arcade(), &roster::spawns(2, 800.0, 600.0), robots);
    battle.set_terrain(Terrain::urban(800.0, 600.0));
    battle
}

/// The `"beam":[…]` arrays in `json`, in order.
fn beams(json: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = json;
    while let Some(i) = rest.find(r#""beam":"#) {
        let after = &rest[i + 7..];
        let end = after
            .find("]]")
            .map_or_else(|| after.find("[]").unwrap() + 2, |e| e + 2);
        out.push(&after[..end]);
        rest = &after[end..];
    }
    out
}

#[test]
fn header_is_version_2() {
    let h = recording::header_json(&duel());
    assert!(h.contains(r#""version":2"#), "{h}");
    let rec = recording::record(duel(), 5);
    assert!(
        rec.starts_with(r#"{"format":"roboverus-recording","version":2,"#)
    );
}

#[test]
fn open_arena_frames_have_no_beam() {
    let robots = vec![
        roster::built_in("SafeShooter").unwrap(),
        roster::built_in("Circler").unwrap(),
    ];
    let battle =
        Battle::new(arcade(), &roster::spawns(2, 800.0, 600.0), robots);
    let json = recording::record(battle, 50);
    assert!(!json.contains("beam"));
    let fixture = include_str!("fixtures/open_arena_0_5_0.json");
    // The 0.5.0 fixture predates the "bullets" header field, which new
    // recordings always write. Compare without it (and with the old version).
    let json = json.replacen(r#""version":2"#, r#""version":1"#, 1).replacen(
        r#","bullets":"arcade""#,
        "",
        1,
    );
    assert_eq!(json, fixture);
}

#[test]
fn urban_frames_have_beams() {
    let json = recording::record(urban_battle("SafeShooter", "Circler"), 200);
    let all = beams(&json);
    assert!(all.len() >= 2 * 200);
    // Skip turn 0's two beams.
    for beam in &all[2..] {
        assert!(beam.starts_with("[["), "{beam}");
        let points = beam.matches("],[").count() + 1;
        assert!(points >= 3, "{beam}");
        for num in beam.split(|c| "[],".contains(c)).filter(|s| !s.is_empty())
        {
            if let Some((_, frac)) = num.split_once('.') {
                assert!(frac.len() <= 1, "{num} in {beam}");
            }
        }
    }
}

#[test]
fn dead_robot_beam_is_empty() {
    let mut battle = urban_battle("SafeShooter", "Spinner");
    let mut turns = 0;
    while battle.state().robots.iter().all(|r| r.alive) {
        battle.step();
        turns += 1;
        assert!(turns < 5000, "nobody died");
    }
    let mut later = Vec::new();
    for _ in 0..3 {
        let events = battle.step();
        later.push(recording::frame(&battle, &events).to_json());
    }
    for frame in &later {
        let b = beams(frame);
        assert_eq!(b.len(), 2, "{frame}");
        assert_eq!(b.iter().filter(|b| **b == "[]").count(), 1, "{b:?}");
    }
}

#[test]
fn tactician_decisions_carry_intent() {
    let json = recording::record(urban_battle("Tactician", "Circler"), 200);
    let marker = r#""intent":{"maneuver":""#;
    let mut count = 0;
    let mut rest = json.as_str();
    while let Some(i) = rest.find(marker) {
        let after = &rest[i + marker.len()..];
        let (m, tail) = after.split_once(r#"","risk":""#).unwrap();
        let (r, tail) = tail.split_once(r#""}"#).unwrap();
        assert!(
            ["search", "patrol", "defensive", "evade", "attack"].contains(&m),
            "{m}"
        );
        assert!(["cautious", "normal", "aggressive"].contains(&r), "{r}");
        count += 1;
        rest = tail;
    }
    assert!(count > 10, "{count}");
    // Robots without intent write no field.
    let plain = recording::record(urban_battle("Circler", "Spinner"), 50);
    assert!(!plain.contains("intent"));
}

#[test]
fn tanks_in_header() {
    let mut battle = duel();
    battle.set_tanks(vec!["k2".into(), "abrams".into()]);
    let json = recording::record(battle, 5);
    assert!(json.contains(r#""tanks":["k2","abrams"]"#));
    assert!(json.contains(r#""version":2"#));
}

#[test]
fn no_tanks_no_field() {
    let json = recording::record(duel(), 5);
    assert!(!json.contains(r#""tanks""#));
}

#[test]
fn live_header_carries_tanks() {
    let mut battle = duel();
    battle.set_tanks(vec!["k2".into(), "abrams".into()]);
    assert!(
        recording::header_json(&battle).contains(r#""tanks":["k2","abrams"]"#)
    );
}

#[test]
fn bullets_in_header() {
    let json = recording::record(duel(), 5);
    assert!(json.contains(r#""bullets":"arcade""#));
    assert!(json.contains(r#""version":2"#));
    let robots = vec![
        roster::built_in("SafeShooter").unwrap(),
        roster::built_in("Spinner").unwrap(),
    ];
    let realistic = Battle::new(
        BattleConfig::default(),
        &roster::spawns(2, 800.0, 600.0),
        robots,
    );
    assert!(
        recording::record(realistic, 5).contains(r#""bullets":"realistic""#)
    );
}

#[test]
fn live_header_carries_bullets() {
    assert!(recording::header_json(&duel()).contains(r#""bullets":"arcade""#));
}
