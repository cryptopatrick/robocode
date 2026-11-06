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

// ---- Task 2: Java turret mode --------------------------------------------

use robocode::physics::TurretMode;

fn java_robot() -> RobotState {
    let mut r = RobotState::new(0, Point::new(400.0, 300.0));
    r.turret.mode = TurretMode::Java;
    r
}

#[test]
fn independent_mode_is_the_default_and_unchanged() {
    let mut r = RobotState::new(0, Point::new(400.0, 300.0));
    assert_eq!(r.turret.mode, TurretMode::Independent);
    r.set_turn_right(10.0);
    let mut b = solo(r);
    b.step();
    assert_eq!((b.robots[0].heading, b.robots[0].gun_heading, b.robots[0].radar_heading), (10.0, 0.0, 0.0));
}

#[test]
fn java_mode_body_turn_drags_gun_and_radar() {
    let mut r = java_robot();
    r.set_turn_right(10.0);
    let mut b = solo(r);
    b.step();
    assert_eq!((b.robots[0].heading, b.robots[0].gun_heading, b.robots[0].radar_heading), (10.0, 10.0, 10.0));
}

#[test]
fn java_mode_adjust_gun_nets_out_the_drag() {
    let mut r = java_robot();
    r.set_adjust_gun_for_robot_turn(true);
    r.set_turn_right(10.0);
    let mut b = solo(r);
    b.step();
    let r = &b.robots[0];
    assert_eq!(r.heading, 10.0);
    assert_eq!(r.gun_heading, 0.0, "gun compensated within the turn");
    assert_eq!(r.radar_heading, 0.0, "radar follows the gun");
}

#[test]
fn java_mode_gun_turn_drags_radar_unless_adjusted() {
    let mut r = java_robot();
    r.set_turn_gun_right(15.0);
    let mut b = solo(r);
    b.step();
    assert_eq!((b.robots[0].gun_heading, b.robots[0].radar_heading), (15.0, 15.0));

    let mut r = java_robot();
    r.set_adjust_radar_for_gun_turn(true);
    r.set_turn_gun_right(15.0);
    let mut b = solo(r);
    b.step();
    assert_eq!((b.robots[0].gun_heading, b.robots[0].radar_heading), (15.0, 0.0));
}

#[test]
fn adjust_radar_for_gun_also_sets_radar_for_body_unless_set() {
    // Java: setAdjustRadarForGunTurn(x) sets AdjustRadarForBodyTurn too, unless set explicitly.
    let mut r = java_robot();
    r.set_adjust_radar_for_gun_turn(true);
    assert!(r.turret.adjust_radar_for_body);
    let mut r = java_robot();
    r.set_adjust_radar_for_robot_turn(false);
    r.set_adjust_radar_for_gun_turn(true);
    assert!(!r.turret.adjust_radar_for_body);
}

#[test]
fn commands_set_turret_mode_and_flags() {
    use robocode::api::TurretMode as ApiMode;
    let mut c = Commands::default();
    c.set_turret_mode(ApiMode::Java);
    c.set_adjust_gun_for_robot_turn(true);
    assert_eq!(c.turret_mode(), Some(ApiMode::Java));
    assert_eq!(c.adjust_gun_for_robot_turn(), Some(true));
}
