//! Map objects: buildings stop robots at their face and absorb bullets; roads
//! and the empty arena change nothing.

use robocode::{Point, Rect};
use robocode::physics::{BattleConfig, BattleState, BulletState, RobotState};
use robocode::rules::ROBOT_RADIUS;
use robocode::terrain::Terrain;

fn robot_at(x: f64, y: f64) -> RobotState {
    let mut robot = RobotState::new(0, Point::new(x, y));
    robot.heading = 90.0;
    robot
}

fn building() -> Rect {
    Rect::new(400.0, 280.0, 60.0, 60.0)
}

#[test]
fn a_segment_hits_a_face_at_the_expected_fraction() {
    let b = building();
    let t = b.first_contact(Point::new(300.0, 300.0), Point::new(500.0, 300.0)).unwrap();
    assert!((t - 0.5).abs() < 1e-9, "the left face is at x=400, halfway along 300→500: {t}");
}

#[test]
fn a_segment_that_only_leaves_or_misses_does_not_hit() {
    let b = building();
    // Starts on the left face and moves away.
    assert!(b.first_contact(Point::new(400.0, 300.0), Point::new(380.0, 300.0)).is_none());
    // Passes above the building.
    assert!(b.first_contact(Point::new(300.0, 200.0), Point::new(500.0, 200.0)).is_none());
    // Stops short of it.
    assert!(b.first_contact(Point::new(300.0, 300.0), Point::new(390.0, 300.0)).is_none());
}

#[test]
fn a_robot_stops_at_the_face_of_a_building() {
    let config = BattleConfig::default();
    let mut battle = BattleState::new(config, vec![robot_at(300.0, 300.0)]);
    battle.terrain = Terrain { buildings: vec![building()], roads: vec![] };
    battle.robots[0].control.distance_remaining = 300.0;
    battle.robots[0].control.max_velocity = 8.0;

    for _ in 0..80 {
        battle.step();
    }

    let x = battle.robots[0].position.x;
    let face = 400.0 - ROBOT_RADIUS;
    assert!((x - face).abs() < 1e-6, "robot stopped at {x}, expected {face}");
    assert_eq!(battle.robots[0].energy, 100.0, "no wall-style damage from a building");
}

#[test]
fn a_robot_that_touches_a_building_can_drive_away() {
    let config = BattleConfig::default();
    let mut battle = BattleState::new(config, vec![robot_at(300.0, 300.0)]);
    battle.terrain = Terrain { buildings: vec![building()], roads: vec![] };
    battle.robots[0].control.distance_remaining = 300.0;
    battle.robots[0].control.max_velocity = 8.0;
    for _ in 0..80 {
        battle.step();
    }
    let stopped = battle.robots[0].position.x;

    battle.robots[0].heading = 270.0;
    battle.robots[0].control.distance_remaining = 40.0;
    for _ in 0..20 {
        battle.step();
    }
    assert!(battle.robots[0].position.x < stopped - 10.0, "the robot is free to back away");
}

#[test]
fn a_building_absorbs_a_bullet_on_its_path() {
    let config = BattleConfig::default();
    let mut battle = BattleState::new(config, vec![robot_at(100.0, 300.0)]);
    battle.terrain = Terrain { buildings: vec![building()], roads: vec![] };
    battle.bullets.push(BulletState::new(1, 0, Point::new(385.0, 310.0), 90.0, 1.0));

    let events = battle.step();

    assert_eq!(events.bullet_hit_obstacles.len(), 1);
    assert!(events.bullet_hits.is_empty());
    assert!(battle.bullets.is_empty(), "the bullet is gone");
}

#[test]
fn roads_and_the_empty_arena_do_not_block_anything() {
    let config = BattleConfig::default();
    let mut battle = BattleState::new(config, vec![robot_at(300.0, 300.0)]);
    battle.terrain = Terrain { buildings: vec![], roads: vec![building()] };
    battle.robots[0].control.distance_remaining = 300.0;
    battle.robots[0].control.max_velocity = 8.0;
    battle.bullets.push(BulletState::new(1, 0, Point::new(385.0, 310.0), 90.0, 1.0));
    let events = battle.step();
    assert!(events.bullet_hit_obstacles.is_empty());
    for _ in 0..80 {
        battle.step();
    }
    assert!(battle.robots[0].position.x > 400.0, "a road doesn't stop a robot");
}

#[test]
fn the_urban_map_keeps_every_spawn_corner_clear() {
    let (w, h) = (800.0, 600.0);
    let corners = [(100.0, 100.0), (w - 100.0, 100.0), (100.0, h - 100.0), (w - 100.0, h - 100.0)];
    let terrain = Terrain::urban(w, h);
    for (x, y) in corners {
        for b in &terrain.buildings {
            assert!(
                b.grow(ROBOT_RADIUS).first_contact(Point::new(x, y), Point::new(x, y)).is_none()
                    && !(x >= b.min.x - ROBOT_RADIUS
                        && x <= b.max.x + ROBOT_RADIUS
                        && y >= b.min.y - ROBOT_RADIUS
                        && y <= b.max.y + ROBOT_RADIUS),
                "spawn ({x},{y}) is inside {b:?}"
            );
        }
    }
}

#[test]
fn every_arena_size_in_the_viewer_fits_its_urban_map() {
    for (w, h) in [(400.0, 400.0), (800.0, 600.0), (1000.0, 1000.0)] {
        let terrain = Terrain::urban(w, h);
        for b in &terrain.buildings {
            assert!(b.min.x >= 0.0 && b.min.y >= 0.0 && b.max.x <= w && b.max.y <= h, "{w}x{h}: {b:?} leaves the arena");
        }
        for spawn in robocode::roster::spawns(4, w, h) {
            for b in &terrain.buildings {
                let g = b.grow(ROBOT_RADIUS);
                let p = spawn.position;
                assert!(!(p.x >= g.min.x && p.x <= g.max.x && p.y >= g.min.y && p.y <= g.max.y), "{w}x{h}: spawn {p:?} is blocked by {b:?}");
            }
        }
    }
}
