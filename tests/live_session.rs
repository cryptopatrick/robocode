use robocode::live::LiveSession;
use robocode::physics::BattleConfig;
use robocode::recording;
use robocode::roster;
use robocode::runner::Battle;

fn duel(a: &str, b: &str) -> Battle {
    let robots = vec![roster::built_in(a).unwrap(), roster::built_in(b).unwrap()];
    Battle::new(BattleConfig::default(), &roster::spawns(2, 800.0, 600.0), robots)
}

#[test]
fn live_frames_equal_precomputed_frames() {
    for (a, b) in [("SafeShooter", "Spinner"), ("Circler", "SittingDuck"), ("SafeShooter", "Circler")] {
        let expected = recording::record(duel(a, b), 5000);
        let mut live = LiveSession::new(duel(a, b), 5000);
        let mut frames = 0;
        while live.step().is_some() {
            frames += 1;
        }
        assert!(live.finished());
        assert_eq!(live.recording(), expected, "{a} vs {b}");
        assert_eq!(frames as u64, live.turns_played());
    }
}

#[test]
fn header_and_first_frame_come_before_any_step() {
    let live = LiveSession::new(duel("SafeShooter", "Spinner"), 5000);
    assert_eq!(live.header(), recording::header_json(&duel("SafeShooter", "Spinner")));
    assert!(live.initial_frame().to_json().contains(r#""turn":0,"#));
    assert!(!live.finished());
    assert_eq!(live.turns_played(), 0);
}

#[test]
fn recording_so_far_is_a_valid_partial_recording() {
    let mut live = LiveSession::new(duel("SafeShooter", "Spinner"), 5000);
    for _ in 0..10 {
        live.step();
    }
    let partial = live.recording();
    assert!(partial.contains(r#""outcome":{"winner":null,"turns":10,"#), "{}", &partial[partial.len() - 120..]);
}

#[test]
fn turn_limit_ends_the_session() {
    let mut live = LiveSession::new(duel("SittingDuck", "SittingDuck"), 7);
    let mut n = 0;
    while live.step().is_some() {
        n += 1;
    }
    assert_eq!((n, live.finished()), (7, true));
    assert_eq!(live.outcome().unwrap().winner, None);
}
