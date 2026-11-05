use robocode::physics::BattleConfig;
use robocode::recording;
use robocode::roster;
use robocode::runner::Battle;

#[test]
fn roster_lists_and_builds() {
    for name in roster::BUILT_IN {
        let robot = roster::built_in(name).unwrap_or_else(|| panic!("{name} builds"));
        assert_eq!(robot.name(), name);
    }
    assert!(roster::built_in("Nope").is_none());
}

#[test]
fn spawns_use_corners() {
    let s = roster::spawns(4, 800.0, 600.0);
    let xy: Vec<(f64, f64)> = s.iter().map(|s| (s.position.x, s.position.y)).collect();
    assert_eq!(xy, [(100.0, 100.0), (700.0, 500.0), (700.0, 100.0), (100.0, 500.0)]);
    assert!(s.iter().all(|s| s.heading == 0.0));
    assert_eq!(roster::spawns(2, 800.0, 600.0).len(), 2);
}

#[test]
fn record_matches_v1_shape() {
    let robots = vec![roster::built_in("SafeShooter").unwrap(), roster::built_in("SittingDuck").unwrap()];
    let battle = Battle::new(BattleConfig::default(), &roster::spawns(2, 800.0, 600.0), robots);
    let json = recording::record(battle, 5000);
    assert!(json.starts_with(r#"{"format":"roboverus-recording","version":1,"robots":[{"id":0,"name":"SafeShooter"}"#));
    assert!(json.contains(r#""battlefield":{"width":800,"height":600}"#));
    assert!(json.contains(r#""outcome":{"winner":0,"turns":364"#));
}

#[test]
fn validate_setup_messages() {
    assert_eq!(recording::validate_setup(1, 800.0, 600.0, 10), Err("need 2 to 4 robots, got 1".into()));
    assert_eq!(recording::validate_setup(2, 300.0, 600.0, 10), Err("arena must be 400..=5000 px per side".into()));
    assert_eq!(recording::validate_setup(2, 800.0, 600.0, 0), Err("max_turns must be 1..=20000".into()));
    assert_eq!(recording::validate_setup(4, 800.0, 600.0, 5000), Ok(()));
}

mod api_robot {
    use robocode::api::{Commands, Robot, Status};
    #[derive(Default)]
    pub struct Tiny;
    impl Robot for Tiny {
        fn name(&self) -> &str {
            "Tiny"
        }
        fn run(&mut self, _s: &Status, c: &mut Commands) {
            c.set_ahead(10.0);
        }
    }
    robocode::robot!(Tiny);
}

mod strategy_robot {
    use robocode::strategy::{Action, Observation, Strategy};
    #[derive(Default)]
    pub struct Still;
    impl Strategy for Still {
        fn name(&self) -> &str {
            "Still"
        }
        fn decide(&mut self, _o: &Observation) -> Action {
            Action::default()
        }
    }
    robocode::robot!(strategy Still);
}

#[test]
fn robot_macro_builds_api_robot() {
    assert_eq!(api_robot::__robocode_robot().name(), "Tiny");
}

#[test]
fn robot_macro_wraps_strategy() {
    assert_eq!(strategy_robot::__robocode_robot().name(), "Still");
}

#[test]
fn record_with_summary_names_the_winner() {
    let robots = vec![roster::built_in("SafeShooter").unwrap(), roster::built_in("SittingDuck").unwrap()];
    let battle = Battle::new(BattleConfig::default(), &roster::spawns(2, 800.0, 600.0), robots);
    let r = recording::record_with_summary(battle, 5000);
    assert_eq!(r.summary(), "SafeShooter wins after 364 turns");
    assert!(r.json.contains(r#""turns":364"#));
}
