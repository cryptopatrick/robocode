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
    // Java: min(|v|, 8); NaN is ignored.
    r.set_max_velocity(-3.0);
    assert_eq!(r.control.max_velocity, 3.0);
    r.set_max_velocity(f64::NAN);
    assert_eq!(r.control.max_velocity, 3.0);
    r.set_max_velocity(f64::INFINITY);
    assert_eq!(r.control.max_velocity, 8.0);
}

#[test]
fn max_turn_rate_ignores_nan() {
    let mut r = RobotState::new(0, Point::new(400.0, 300.0));
    r.set_max_turn_rate(4.0);
    r.set_max_turn_rate(f64::NAN);
    assert_eq!(r.control.max_turn_rate, 4.0);
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

// ---- Task 3: robot-robot collisions -------------------------------------

use robocode::physics::{RobotCollision, TurnEvents};

/// A drives right (heading 90) toward B, which sits still 40 px away.
fn ram() -> BattleState {
    let mut a = RobotState::new(0, Point::new(400.0, 300.0));
    a.heading = 90.0;
    a.set_ahead(100.0);
    let b = RobotState::new(1, Point::new(440.0, 300.0));
    BattleState::new(BattleConfig::default(), vec![a, b])
}

fn step_until_collision(b: &mut BattleState) -> TurnEvents {
    for _ in 0..20 {
        let e = b.step();
        if !e.robot_collisions.is_empty() {
            return e;
        }
    }
    panic!("no collision");
}

#[test]
fn ramming_costs_both_and_stops_the_rammer() {
    let mut b = ram();
    step_until_collision(&mut b);
    let (a, v) = (&b.robots[0], &b.robots[1]);
    assert_eq!(a.energy, 100.0 - 0.6);
    assert_eq!(v.energy, 100.0 - 0.6);
    assert_eq!(a.velocity, 0.0);
    assert_eq!(a.control.distance_remaining, 0.0);
    assert!(v.position.x - a.position.x >= 36.0, "rammer moved back out: {}", v.position.x - a.position.x);
}

#[test]
fn collision_events_mark_fault() {
    let mut b = ram();
    let e = step_until_collision(&mut b);
    let pairs: Vec<(usize, usize, bool)> = e.robot_collisions.iter().map(|c: &RobotCollision| (c.robot_id, c.other_robot_id, c.at_fault)).collect();
    assert_eq!(pairs, [(0, 1, true), (1, 0, false)]);
}

#[test]
fn reversing_into_a_robot_is_also_at_fault() {
    let mut a = RobotState::new(0, Point::new(400.0, 300.0));
    a.heading = 270.0; // facing left, B is behind (to the right)
    a.set_ahead(-100.0);
    let b = RobotState::new(1, Point::new(440.0, 300.0));
    let mut bs = BattleState::new(BattleConfig::default(), vec![a, b]);
    let e = step_until_collision(&mut bs);
    assert!(e.robot_collisions.iter().any(|c| c.robot_id == 0 && c.at_fault));
}

#[test]
fn collision_skips_the_next_body_turn() {
    let mut b = ram();
    step_until_collision(&mut b);
    b.robots[0].set_turn_right(30.0);
    b.step();
    assert_eq!(b.robots[0].heading, 90.0);
    b.step();
    assert!(b.robots[0].heading > 90.0, "turns again once clear");
}

#[test]
fn ramming_can_kill() {
    let mut b = ram();
    b.robots[1].energy = 0.5;
    let e = step_until_collision(&mut b);
    assert!(!b.robots[1].alive);
    assert_eq!(e.robot_deaths, vec![1]);
}

// ---- Task 4: bullet-on-bullet collisions --------------------------------

/// A (bottom) fires up, B (top) fires down: the bullets meet in the middle.
fn duel_of_bullets() -> BattleState {
    let mut a = RobotState::new(0, Point::new(400.0, 500.0));
    a.set_fire(1.0);
    let mut b = RobotState::new(1, Point::new(400.0, 100.0));
    b.gun_heading = 180.0;
    b.set_fire(3.0);
    BattleState::new(BattleConfig::default(), vec![a, b])
}

#[test]
fn bullets_fired_at_each_other_collide() {
    let mut bs = duel_of_bullets();
    let (mut hits, mut misses, mut collisions) = (0, 0, Vec::new());
    for _ in 0..40 {
        let e = bs.step();
        hits += e.bullet_hits.len();
        misses += e.bullet_misses.len();
        collisions.extend(e.bullet_hit_bullets);
    }
    assert_eq!((hits, misses, collisions.len()), (0, 0, 1));
    let c = &collisions[0];
    let mut owners = [c.owner, c.other_owner];
    owners.sort();
    assert_eq!(owners, [0, 1]);
    assert!(bs.bullets.is_empty());
    assert_eq!((bs.robots[0].energy, bs.robots[1].energy), (100.0 - 1.0, 100.0 - 3.0));
}

#[test]
fn parallel_bullets_do_not_collide() {
    let mut a = RobotState::new(0, Point::new(300.0, 500.0));
    a.set_fire(1.0);
    let mut b = RobotState::new(1, Point::new(500.0, 500.0));
    b.set_fire(1.0);
    let mut bs = BattleState::new(BattleConfig::default(), vec![a, b]);
    let (mut misses, mut collisions) = (0, 0);
    for _ in 0..40 {
        let e = bs.step();
        misses += e.bullet_misses.len();
        collisions += e.bullet_hit_bullets.len();
    }
    assert_eq!((misses, collisions), (2, 0));
}

#[test]
fn commands_expose_every_flag_they_carry() {
    let mut c = Commands::default();
    c.set_adjust_radar_for_robot_turn(true);
    c.set_adjust_radar_for_gun_turn(false);
    assert_eq!((c.adjust_radar_for_robot_turn(), c.adjust_radar_for_gun_turn()), (Some(true), Some(false)));
}
