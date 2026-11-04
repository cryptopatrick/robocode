use robocode::physics::{BattleConfig, BattleState, RobotState};
use robocode::{Battlefield, Point};

fn big_field() -> BattleConfig {
    BattleConfig { battlefield: Battlefield::new(2000.0, 2000.0), ..BattleConfig::default() }
}

/// Robot 0 at `a` with its radar sweeping from 0° by `sweep` degrees; robot 1 at `b`.
fn scan_once(a: Point, sweep: f64, b: Point) -> Vec<robocode::physics::ScannedRobot> {
    let mut scanner = RobotState::new(0, a);
    scanner.set_turn_radar_right(sweep);
    let target = RobotState::new(1, b);
    let mut battle = BattleState::new(big_field(), vec![scanner, target]);
    battle.step().scans.into_iter().filter(|s| s.scanner == 0).collect()
}

#[test]
fn radar_sweep_finds_robot_in_arc() {
    let a = Point::new(1000.0, 1000.0);
    let scans = scan_once(a, 45.0, a.translate(20.0, 200.0));
    assert_eq!(scans.len(), 1);
    let s = &scans[0];
    assert_eq!((s.scanner, s.target), (0, 1));
    assert!((s.bearing - 20.0).abs() < 1e-6, "bearing {}", s.bearing);
    assert!((s.distance - 200.0).abs() < 1e-6, "distance {}", s.distance);
    assert_eq!(s.energy, 100.0);
}

#[test]
fn robot_outside_arc_not_scanned() {
    let a = Point::new(1000.0, 1000.0);
    assert!(scan_once(a, 45.0, a.translate(90.0, 200.0)).is_empty());
}

#[test]
fn robot_beyond_radius_not_scanned() {
    let a = Point::new(300.0, 1500.0);
    assert!(scan_once(a, 45.0, a.translate(20.0, 1201.0)).is_empty());
    assert_eq!(scan_once(a, 45.0, a.translate(20.0, 1199.0)).len(), 1);
}

#[test]
fn stationary_radar_still_sees_robot_on_line() {
    let a = Point::new(1000.0, 1000.0);
    assert_eq!(scan_once(a, 0.0, a.translate(0.0, 300.0)).len(), 1);
}

#[test]
fn sweep_across_zero_finds_robot() {
    // Radar starts at 350° and turns right 20° to 10°; target at bearing 0°.
    let a = Point::new(1000.0, 1000.0);
    let mut scanner = RobotState::new(0, a);
    scanner.radar_heading = 350.0;
    scanner.set_turn_radar_right(20.0);
    let target = RobotState::new(1, a.translate(0.0, 300.0));
    let mut battle = BattleState::new(big_field(), vec![scanner, target]);
    assert_eq!(battle.step().scans.len(), 1);
}

#[test]
fn overlapping_robots_scan_without_nan() {
    let a = Point::new(1000.0, 1000.0);
    let scans = scan_once(a, 45.0, a);
    assert_eq!(scans.len(), 1);
    assert!(scans[0].bearing.is_finite());
    assert_eq!(scans[0].distance, 0.0);
}

#[test]
fn dead_robots_neither_scan_nor_are_scanned() {
    let a = Point::new(1000.0, 1000.0);
    let mut scanner = RobotState::new(0, a);
    scanner.set_turn_radar_right(45.0);
    let mut target = RobotState::new(1, a.translate(20.0, 200.0));
    target.kill();
    let mut battle = BattleState::new(big_field(), vec![scanner.clone(), target]);
    assert!(battle.step().scans.is_empty());

    scanner.kill();
    let alive = RobotState::new(1, a.translate(20.0, 200.0));
    let mut battle = BattleState::new(big_field(), vec![scanner, alive]);
    assert!(battle.step().scans.is_empty());
}
