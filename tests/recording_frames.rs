use robocode::physics::BattleConfig;
use robocode::recording::{self, RecordingBuilder};
use robocode::roster;
use robocode::runner::Battle;

fn duel() -> Battle {
    let robots = vec![roster::built_in("SafeShooter").unwrap(), roster::built_in("Spinner").unwrap()];
    Battle::new(BattleConfig::default(), &roster::spawns(2, 800.0, 600.0), robots)
}

#[test]
fn frames_rebuild_the_same_recording() {
    let expected = recording::record(duel(), 5000);

    let mut battle = duel();
    let mut builder = RecordingBuilder::new(&battle);
    builder.push(&recording::initial_frame(&battle));
    let mut turns = 0;
    while battle.state().robots.iter().filter(|r| r.alive).count() > 1 && turns < 5000 {
        let events = battle.step();
        builder.push(&recording::frame(&battle, &events));
        turns += 1;
    }
    let outcome = battle.run(0);
    assert_eq!(builder.finish(&battle, &outcome).json, expected);
}

#[test]
fn header_is_a_v1_object() {
    let h = recording::header_json(&duel());
    assert_eq!(
        h,
        r#"{"format":"roboverus-recording","version":1,"robots":[{"id":0,"name":"SafeShooter"},{"id":1,"name":"Spinner"}],"battlefield":{"width":800,"height":600}}"#
    );
}

#[test]
fn frame_json_holds_turn_and_decisions() {
    let mut battle = duel();
    let events = battle.step();
    let f = recording::frame(&battle, &events).to_json();
    assert!(f.starts_with(r#"{"turn":{"battle_id":"viewer","turn":1,"#), "{}", &f[..80]);
    assert!(f.contains(r#""decisions":[{"energy":100000"#), "{}", &f[f.len() - 300..]);
}
