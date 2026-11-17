use robocode::strategy::maneuver::memory::ManeuverMemory;
use robocode::strategy::maneuver::{fire_filter, Intent, Maneuver, Risk};
use robocode::strategy::{Observation, TargetInfo};
use robocode::units::{Angle, Energy, GunHeat, Power};

fn obs() -> Observation {
    Observation {
        turn: 10,
        energy: Energy(100_000),
        gun_heat: GunHeat(0),
        heading: Angle(0),
        gun_heading: Angle(0),
        radar_heading: Angle(0),
        velocity: 0,
        x: 400,
        y: 300,
        field_width: 800,
        field_height: 600,
        target: None,
    }
}

fn with_target(
    bearing: i32,
    distance: i32,
    energy: u32,
    velocity: i32,
    age: u32,
) -> Observation {
    let mut o = obs();
    o.target = Some(TargetInfo {
        bearing: Angle(bearing),
        distance,
        energy: Energy(energy),
        heading: Angle(0),
        velocity,
        age,
    });
    o
}

#[test]
fn risk_params_match_the_spec() {
    let c = Risk::Cautious.params();
    assert_eq!(
        (
            c.reserve,
            c.engage_distance,
            c.aim_tolerance,
            c.shot_power,
            c.keep_clear,
            c.evade_turns,
            c.stale_after
        ),
        (Energy(40000), 450, 20, Power(1000), 80, 20, 10)
    );
    let n = Risk::Normal.params();
    assert_eq!(
        (
            n.reserve,
            n.engage_distance,
            n.aim_tolerance,
            n.shot_power,
            n.keep_clear,
            n.evade_turns,
            n.stale_after
        ),
        (Energy(20000), 300, 40, Power(2000), 50, 12, 20)
    );
    let a = Risk::Aggressive.params();
    assert_eq!(
        (
            a.reserve,
            a.engage_distance,
            a.aim_tolerance,
            a.shot_power,
            a.keep_clear,
            a.evade_turns,
            a.stale_after
        ),
        (Energy(5000), 150, 80, Power(3000), 25, 6, 30)
    );
    assert_eq!(Maneuver::Search.name(), "search");
    assert_eq!(Risk::Aggressive.name(), "aggressive");
    let i = Intent { maneuver: Maneuver::Evade, risk: Risk::Normal };
    assert_eq!(i, i);
}

#[test]
fn fire_filter_needs_a_fresh_affordable_target() {
    assert_eq!(
        fire_filter(&with_target(0, 300, 50_000, 0, 1), Power(2000)),
        None
    );
    // our own energy is what limits the shot
    let mut poor = with_target(0, 300, 50_000, 0, 0);
    poor.energy = Energy(2000);
    assert_eq!(fire_filter(&poor, Power(2000)), None);
    // a nearly dead target does not block a shot when we have plenty
    assert_eq!(
        fire_filter(&with_target(0, 300, 500, 0, 0), Power(2000)),
        Some(Power(2000))
    );
    assert_eq!(
        fire_filter(&with_target(0, 300, 50_000, 0, 0), Power(2000)),
        Some(Power(2000))
    );
    assert_eq!(fire_filter(&obs(), Power(2000)), None);
}

/// Observe `first`, then `second`; return whether `second` detected a shot.
fn detect(first: Observation, second: Observation) -> bool {
    let mut m = ManeuverMemory::new(1);
    m.observe(&first);
    m.observe(&second)
}

#[test]
fn shot_detection_table() {
    let before = with_target(0, 300, 50_000, 0, 0);
    // plain drop of 2.0
    assert!(detect(before, with_target(0, 300, 48_000, 0, 0)));
    // drop too small / too large
    assert!(!detect(before, with_target(0, 300, 49_950, 0, 0)));
    assert!(!detect(before, with_target(0, 300, 46_000, 0, 0)));
    // own energy rose: our bullet hit
    let mut hit = with_target(0, 300, 48_000, 0, 0);
    hit.energy = Energy(103_000);
    assert!(!detect(before, hit));
    // target stopped at a wall: bearing 900 (right), distance 380 -> x = 780
    let moving = with_target(900, 380, 50_000, 80, 0);
    assert!(!detect(moving, with_target(900, 380, 48_000, 0, 0)));
    // collision range
    assert!(!detect(before, with_target(0, 30, 48_000, 0, 0)));
    // previous scan stale
    let stale = with_target(0, 300, 50_000, 0, 3);
    assert!(!detect(stale, with_target(0, 300, 48_000, 0, 0)));
}

#[test]
fn rng_is_seeded() {
    let bits = |seed| {
        let mut m = ManeuverMemory::new(seed);
        (0..64).map(|_| m.next_bit()).collect::<Vec<_>>()
    };
    assert_eq!(bits(7), bits(7));
    assert_ne!(bits(7), bits(8));
}

#[test]
fn fresh_stale_fresh_does_not_detect() {
    let mut m = ManeuverMemory::new(1);
    assert!(!m.observe(&with_target(0, 300, 50_000, 0, 0)));
    assert!(!m.observe(&with_target(0, 300, 50_000, 0, 3)));
    assert!(!m.observe(&with_target(0, 300, 48_000, 0, 0)));
}

// ---- the five maneuvers ----

use robocode::strategy::maneuver::{bearing_to, moves};
use robocode::strategy::Action;
use robocode::units::angle_diff;

const ALL: [Maneuver; 5] = [
    Maneuver::Search,
    Maneuver::Patrol,
    Maneuver::Defensive,
    Maneuver::Evade,
    Maneuver::Attack,
];

#[test]
fn search_spins_the_radar_full_speed() {
    let mut mem = ManeuverMemory::new(1);
    let a = moves::search(&obs(), &mut mem, &Risk::Normal.params());
    assert_eq!(a.turn_radar, Some(450));
    assert_eq!(a.fire, None);
}

#[test]
fn search_heads_for_the_centre() {
    let mut o = obs();
    o.x = 100;
    o.y = 100;
    let mut mem = ManeuverMemory::new(1);
    let a = moves::search(&o, &mut mem, &Risk::Normal.params());
    let want = angle_diff(bearing_to(&o, 400, 300), o.heading);
    assert!((a.turn_body.unwrap() - want).abs() <= 10);
    assert!(a.ahead.unwrap() > 0);
}

#[test]
fn patrol_advances_waypoints() {
    let p = Risk::Normal.params();
    let mut o = obs();
    // Waypoint 0 is (cx, keep_clear + 40).
    o.x = 400;
    o.y = p.keep_clear + 40 + 5;
    let mut mem = ManeuverMemory::new(1);
    assert_eq!(mem.waypoint, 0);
    let a = moves::patrol(&o, &mut mem, &p);
    assert_eq!(mem.waypoint, 1);
    assert_eq!(a.fire, None);
}

#[test]
fn evade_drives_at_right_angles() {
    let mut mem = ManeuverMemory::new(1);
    let a = moves::evade(
        &with_target(0, 300, 50_000, 0, 0),
        &mut mem,
        &Risk::Normal.params(),
    );
    assert!((a.turn_body.unwrap().abs() - 900).abs() <= 1);
    assert!(a.ahead.unwrap() != 0);
    assert_eq!(a.fire, None);
}

#[test]
fn evade_direction_is_seeded() {
    let run = |seed| {
        let mut mem = ManeuverMemory::new(seed);
        (0..10)
            .map(|_| {
                mem.turns_in_maneuver = 0;
                moves::evade(
                    &with_target(0, 300, 50_000, 0, 0),
                    &mut mem,
                    &Risk::Normal.params(),
                );
                mem.evade_dir
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(7), run(7));
    let dirs = run(7);
    assert!(dirs.iter().any(|&d| d != dirs[0]));
}

#[test]
fn evade_keeps_its_direction_after_entry() {
    let mut mem = ManeuverMemory::new(3);
    let o = with_target(0, 300, 50_000, 0, 0);
    moves::evade(&o, &mut mem, &Risk::Normal.params());
    let d = mem.evade_dir;
    mem.turns_in_maneuver = 4;
    moves::evade(&o, &mut mem, &Risk::Normal.params());
    assert_eq!(mem.evade_dir, d);
    assert_eq!(mem.turns_in_maneuver, 4);
}

#[test]
fn attack_closes_and_fires_when_aimed() {
    let mut mem = ManeuverMemory::new(1);
    let a = moves::attack(
        &with_target(0, 500, 50_000, 0, 0),
        &mut mem,
        &Risk::Normal.params(),
    );
    assert_eq!(a.ahead, Some(100));
    assert_eq!(a.fire, Some(Power(2000)));
}

#[test]
fn defensive_fires_low_power_only() {
    let mut mem = ManeuverMemory::new(1);
    let a = moves::defensive(
        &with_target(0, 500, 50_000, 0, 0),
        &mut mem,
        &Risk::Aggressive.params(),
    );
    assert_eq!(a.fire, Some(Power(1000)));
}

#[test]
fn every_maneuver_turns_the_radar() {
    for m in ALL {
        for o in [obs(), with_target(900, 300, 50_000, 0, 0)] {
            let mut mem = ManeuverMemory::new(1);
            let a: Action = m.run(&o, &mut mem, &Risk::Normal.params());
            assert!(
                matches!(a.turn_radar, Some(x) if x != 0),
                "{} left the radar alone",
                m.name()
            );
        }
    }
}
