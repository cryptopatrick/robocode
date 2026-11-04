use robocode::Point;
use robocode::physics::{BattleConfig, BattleState, RobotState};

#[test]
fn robot_moves_forward_and_stops_near_target_distance() {
    let mut robot = RobotState::new(0, Point::new(400.0, 300.0));
    robot.set_ahead(100.0);
    let mut battle = BattleState::new(BattleConfig::default(), vec![robot]);
    for _ in 0..40 {
        battle.step();
    }
    let robot = &battle.robots[0];
    assert!(robot.control.distance_remaining.abs() < 0.1);
    let traveled = 300.0 - robot.position.y;
    assert!(
        (traveled - 100.0).abs() < 5.0,
        "robot traveled {:.2} instead of 100",
        traveled
    );
}

#[test]
fn firing_creates_bullet_and_cools_down() {
    let mut robot = RobotState::new(0, Point::new(400.0, 300.0));
    robot.set_fire(2.5);
    let mut battle = BattleState::new(BattleConfig::default(), vec![robot]);
    let events = battle.step();
    assert_eq!(events.bullet_fired.len(), 1);
    assert!(battle.robots[0].gun_heat > 0.0);
    for _ in 0..30 {
        battle.step();
    }
    assert!(battle.robots[0].gun_heat < 0.1);
}

#[test]
fn bullet_hit_reduces_energy() {
    let mut shooter = RobotState::new(0, Point::new(400.0, 550.0));
    shooter.gun_heading = 0.0;
    let target = RobotState::new(1, Point::new(400.0, 150.0));
    let mut battle =
        BattleState::new(BattleConfig::default(), vec![shooter, target]);
    battle.robots[0].set_fire(3.0);
    let mut hit_observed = false;
    for _ in 0..80 {
        let events = battle.step();
        if !events.bullet_hits.is_empty() {
            hit_observed = true;
            break;
        }
    }
    assert!(hit_observed, "expected bullet hit event");
    assert!(battle.robots[1].energy < 100.0);
}

#[test]
fn turn_snapshot_serializes_to_json() {
    let mut battle = BattleState::new(
        BattleConfig::default(),
        vec![RobotState::new(0, Point::new(300.0, 300.0))],
    );
    let events = battle.step();
    let snapshot = battle.snapshot("snapshot-test", &events);
    let json = snapshot.to_json_string();
    assert!(json.contains("snapshot-test"));
    assert!(json.contains("\"robots\""));
}

#[test]
fn snapshot_json_writes_non_finite_numbers_as_null() {
    use robocode::physics::TurnEvents;
    let mut robot = RobotState::new(0, Point::new(400.0, 300.0));
    robot.energy = f64::NAN;
    robot.velocity = f64::INFINITY;
    let battle = BattleState::new(BattleConfig::default(), vec![robot]);
    let json = battle.snapshot("t", &TurnEvents::default()).to_json_string();
    assert!(json.contains(r#""energy":null"#), "{json}");
    assert!(json.contains(r#""velocity":null"#), "{json}");
    assert!(!json.contains("NaN") && !json.contains("inf"), "{json}");
}

#[test]
fn bullet_leaving_field_is_a_miss() {
    let mut robot = RobotState::new(0, Point::new(400.0, 300.0));
    robot.set_fire(3.0); // gun points up (0°), nothing in the way
    let mut battle = BattleState::new(BattleConfig::default(), vec![robot]);
    let misses: Vec<_> = (0..40).flat_map(|_| battle.step().bullet_misses).collect();
    assert_eq!(misses.len(), 1);
    assert_eq!(misses[0].shooter, 0);
    assert_eq!(misses[0].power, 3.0);
}

#[test]
fn hit_is_not_a_miss() {
    let mut shooter = RobotState::new(0, Point::new(400.0, 500.0));
    shooter.set_fire(1.0);
    let target = RobotState::new(1, Point::new(400.0, 200.0));
    let mut battle = BattleState::new(BattleConfig::default(), vec![shooter, target]);
    let (mut hits, mut misses) = (0, 0);
    for _ in 0..40 {
        let e = battle.step();
        hits += e.bullet_hits.len();
        misses += e.bullet_misses.len();
    }
    assert_eq!((hits, misses), (1, 0));
}
