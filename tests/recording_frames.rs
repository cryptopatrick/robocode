use robocode::physics::BattleConfig;
use robocode::recording::{self, RecordingBuilder};
use robocode::roster;
use robocode::runner::Battle;

fn duel() -> Battle {
    let robots = vec![
        roster::built_in("SafeShooter").unwrap(),
        roster::built_in("Spinner").unwrap(),
    ];
    Battle::new(
        BattleConfig::default(),
        &roster::spawns(2, 800.0, 600.0),
        robots,
    )
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
        r#"{"format":"roboverus-recording","version":2,"robots":[{"id":0,"name":"SafeShooter"},{"id":1,"name":"Spinner"}],"battlefield":{"width":800,"height":600}}"#
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
    let mut battle = Battle::new(
        BattleConfig::default(),
        &roster::spawns(2, 800.0, 600.0),
        robots,
    );
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
    let battle = Battle::new(
        BattleConfig::default(),
        &roster::spawns(2, 800.0, 600.0),
        robots,
    );
    let json = recording::record(battle, 50);
    assert!(!json.contains("beam"));
    let fixture = include_str!("fixtures/open_arena_0_5_0.json");
    assert_eq!(json.replacen(r#""version":2"#, r#""version":1"#, 1), fixture);
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
