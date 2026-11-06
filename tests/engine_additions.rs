//! Engine features Java AdvancedRobots rely on.

use robocode::Point;
use robocode::api::Commands;
use robocode::physics::{BattleConfig, BattleState, RobotState};

fn solo(robot: RobotState) -> BattleState {
    BattleState::new(BattleConfig::default(), vec![robot])
}

// ---- Task 1: max velocity and max turn rate ------------------------------

#[test]
fn max_velocity_caps_speed() {
    let mut r = RobotState::new(0, Point::new(400.0, 550.0));
    r.set_max_velocity(5.0);
    r.set_ahead(450.0);
    let mut b = solo(r);
    let mut fastest: f64 = 0.0;
    for _ in 0..30 {
        b.step();
        fastest = fastest.max(b.robots[0].velocity.abs());
    }
    assert_eq!(fastest, 5.0);
}

#[test]
fn max_velocity_is_clamped_to_rules() {
    let mut r = RobotState::new(0, Point::new(400.0, 300.0));
    r.set_max_velocity(20.0);
    assert_eq!(r.control.max_velocity, 8.0);
    r.set_max_velocity(-3.0);
    assert_eq!(r.control.max_velocity, 0.0);
}

#[test]
fn max_turn_rate_caps_body_turn() {
    let mut r = RobotState::new(0, Point::new(400.0, 300.0));
    r.set_max_turn_rate(4.0);
    r.set_turn_right(90.0);
    let mut b = solo(r);
    b.step();
    assert_eq!(b.robots[0].heading, 4.0);
}

#[test]
fn commands_carry_caps() {
    let mut c = Commands::default();
    c.set_max_velocity(3.5);
    c.set_max_turn_rate(2.0);
    assert_eq!((c.max_velocity(), c.max_turn_rate()), (Some(3.5), Some(2.0)));
}
