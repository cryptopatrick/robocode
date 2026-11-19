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

// ---- Earliest contact along the bullet's path ----

fn duel(victim_x: f64, buildings: Vec<robocode::Rect>) -> BattleState {
    let mut victim = RobotState::new(1, Point::new(victim_x, 300.0));
    victim.gun_heat = 3.0; // the victim does not shoot back
    let mut battle = BattleState::new(
        BattleConfig::default(),
        vec![shooter(100.0, 300.0, 90.0), victim],
    );
    battle.terrain = robocode::terrain::Terrain { buildings, roads: vec![] };
    battle.robots[0].control.fire_power = Some(1.0);
    battle
}

#[test]
fn fast_bullet_hits_robot_it_would_jump() {
    // The bullet steps 100 -> 400 in one turn; the robot sits midway at 250,
    // 150 px from where the bullet lands.
    let mut battle = duel(250.0, vec![]);
    let events = battle.step();
    assert_eq!(events.bullet_hits.len(), 1, "{events:?}");
    assert_eq!(events.bullet_hits[0].victim, 1);
    assert!(battle.bullets.is_empty());
}

#[test]
fn robot_before_building_is_hit() {
    let wall = robocode::Rect::new(330.0, 270.0, 60.0, 60.0);
    let mut battle = duel(250.0, vec![wall]);
    let events = battle.step();
    assert_eq!(events.bullet_hits.len(), 1, "{events:?}");
    assert!(events.bullet_hit_obstacles.is_empty(), "{events:?}");
}

#[test]
fn building_before_robot_absorbs() {
    let wall = robocode::Rect::new(160.0, 270.0, 40.0, 60.0);
    let mut battle = duel(300.0, vec![wall]);
    let events = battle.step();
    assert_eq!(events.bullet_hit_obstacles.len(), 1, "{events:?}");
    assert!(events.bullet_hits.is_empty(), "{events:?}");
    assert!(battle.robots[1].energy == 100.0);
}

#[test]
fn arcade_outcomes_unchanged() {
    use robocode::{recording, roster, runner::Battle};
    let robots = vec![
        roster::built_in("SafeShooter").unwrap(),
        roster::built_in("Circler").unwrap(),
    ];
    // Pinned to arcade: the 0.5.0 fixture is a classic-speed battle.
    let config = BattleConfig {
        bullet_mode: BulletMode::Arcade,
        ..BattleConfig::default()
    };
    let battle = Battle::new(config, &roster::spawns(2, 800.0, 600.0), robots);
    let json = recording::record(battle, 50);
    let fixture = include_str!("fixtures/open_arena_0_5_0.json");
    // The 0.5.0 fixture predates the version bump and the "bullets" header
    // field; compare without them.
    let json = json.replacen(r#""version":2"#, r#""version":1"#, 1).replacen(
        r#","bullets":"arcade""#,
        "",
        1,
    );
    assert_eq!(json, fixture);
}

// ---- Robots see the mode ----

#[test]
fn observation_reports_the_battle_mode() {
    let robot = RobotState::new(0, Point::new(400.0, 300.0));
    for mode in [BulletMode::Realistic, BulletMode::Arcade] {
        let config =
            BattleConfig { bullet_mode: mode, ..BattleConfig::default() };
        let obs = robocode::runner::observe(&robot, &config, 1, None);
        assert_eq!(obs.bullet_mode, mode);
    }
}

#[test]
fn bullet_speed_for_gives_the_mode_speed() {
    use robocode::units::Power;
    let robot = RobotState::new(0, Point::new(400.0, 300.0));
    let observe = |mode| {
        let config =
            BattleConfig { bullet_mode: mode, ..BattleConfig::default() };
        robocode::runner::observe(&robot, &config, 1, None)
    };
    let realistic = observe(BulletMode::Realistic);
    let arcade = observe(BulletMode::Arcade);
    for p in [Power(100), Power(1000), Power(3000)] {
        assert_eq!(realistic.bullet_speed_for(p), 300.0);
        assert_eq!(arcade.bullet_speed_for(p), 20.0 - 3.0 * p.to_f64());
    }
}

// ---- Fix round 1 ----

#[test]
fn realistic_hit_radius_matches_arcade() {
    // The path passes 20 px from the robot's centre: inside the arcade hit
    // radius (robot radius 18 + bullet radius 3).
    let mut victim = RobotState::new(1, Point::new(250.0, 320.0));
    victim.gun_heat = 3.0;
    let mut battle = BattleState::new(
        BattleConfig::default(),
        vec![shooter(100.0, 300.0, 90.0), victim],
    );
    battle.robots[0].control.fire_power = Some(1.0);
    let events = battle.step();
    assert_eq!(events.bullet_hits.len(), 1, "{events:?}");
}

#[test]
fn bullets_collide_before_a_farther_robot() {
    // A fires right along y=300, B fires down along x=250; they cross at
    // (250, 300), halfway along A's path. C sits farther along A's path.
    let mut b = shooter(250.0, 50.0, 180.0);
    b.id = 1;
    let mut c = RobotState::new(2, Point::new(380.0, 300.0));
    c.gun_heat = 3.0;
    let mut battle = BattleState::new(
        BattleConfig::default(),
        vec![shooter(100.0, 300.0, 90.0), b, c],
    );
    battle.robots[0].control.fire_power = Some(1.0);
    battle.robots[1].control.fire_power = Some(1.0);
    let events = battle.step();
    assert_eq!(events.bullet_fired.len(), 2);
    assert_eq!(events.bullet_hit_bullets.len(), 1, "{events:?}");
    assert!(events.bullet_hits.is_empty(), "{events:?}");
    assert_eq!(battle.robots[2].energy, 100.0);
}

#[test]
fn shooter_is_not_hit_by_its_own_bullet() {
    // The swept path starts inside the shooter's own circle.
    let mut battle = BattleState::new(
        BattleConfig::default(),
        vec![shooter(100.0, 300.0, 90.0)],
    );
    battle.robots[0].control.fire_power = Some(1.0);
    let events = battle.step();
    assert!(events.bullet_hits.is_empty(), "{events:?}");
    assert_eq!(battle.bullets.len(), 1, "the bullet flies on");
}
