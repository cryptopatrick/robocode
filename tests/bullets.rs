//! Bullet modes: realistic (default, 300 px per turn) and arcade (Robocode).

use robocode::physics::{BattleConfig, BattleState, RobotState};
use robocode::rules::{self, REALISTIC_BULLET_SPEED};
use robocode::{BulletMode, Point};

fn shooter(x: f64, y: f64, gun_heading: f64) -> RobotState {
    let mut robot = RobotState::new(0, Point::new(x, y));
    robot.gun_heading = gun_heading;
    robot.gun_heat = 0.0;
    robot
}

#[test]
fn default_mode_is_realistic() {
    assert_eq!(BulletMode::default(), BulletMode::Realistic);
    assert_eq!(BattleConfig::default().bullet_mode, BulletMode::Realistic);
    assert_eq!(BulletMode::Realistic.name(), "realistic");
    assert_eq!(BulletMode::Arcade.name(), "arcade");
    assert_eq!("arcade".parse::<BulletMode>(), Ok(BulletMode::Arcade));
    assert_eq!("realistic".parse::<BulletMode>(), Ok(BulletMode::Realistic));
    assert!("slow".parse::<BulletMode>().is_err());
}

#[test]
fn realistic_speed_is_constant() {
    assert_eq!(REALISTIC_BULLET_SPEED, 300.0);
    assert_eq!(rules::bullet_speed(0.1, BulletMode::Realistic), 300.0);
    assert_eq!(rules::bullet_speed(3.0, BulletMode::Realistic), 300.0);
}

#[test]
fn arcade_speed_matches_robocode() {
    for power in [0.1, 1.0, 2.5, 3.0] {
        assert_eq!(
            rules::bullet_speed(power, BulletMode::Arcade),
            20.0 - 3.0 * power
        );
    }
}

#[test]
fn realistic_bullet_moves_300_per_turn() {
    let mut battle = BattleState::new(
        BattleConfig::default(),
        vec![shooter(100.0, 300.0, 90.0)],
    );
    battle.robots[0].control.fire_power = Some(1.0);
    let events = battle.step();
    assert_eq!(events.bullet_fired.len(), 1);
    let bullet = &battle.bullets[0];
    assert!((bullet.position.x - 400.0).abs() < 1e-9, "{:?}", bullet.position);
    assert!((bullet.position.y - 300.0).abs() < 1e-9, "{:?}", bullet.position);
    assert_eq!(bullet.speed, 300.0);
}
