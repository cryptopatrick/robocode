use robocode::strategy::maneuver::memory::ManeuverMemory;
use robocode::strategy::maneuver::{Intent, Maneuver, Risk, fire_filter};
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
        bullet_mode: robocode::BulletMode::Arcade,
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

use robocode::strategy::Action;
use robocode::strategy::maneuver::{bearing_to, moves};
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

fn run_with_velocity(
    m: Maneuver,
    mem: &mut ManeuverMemory,
    velocity: i32,
) -> Action {
    let o = Observation { velocity, ..with_target(0, 500, 50_000, 0, 0) };
    m.run(&o, mem, &Risk::Normal.params())
}

#[test]
fn stuck_robot_backs_off_and_turns() {
    for m in
        [Maneuver::Search, Maneuver::Patrol, Maneuver::Attack, Maneuver::Evade]
    {
        let mut mem = ManeuverMemory::new(5);
        for _ in 0..3 {
            let a = run_with_velocity(m, &mut mem, 0);
            assert_ne!(a.ahead, Some(-60), "{m:?} not stuck yet");
        }
        let a = run_with_velocity(m, &mut mem, 0);
        assert_eq!(a.ahead, Some(-60), "{m:?}");
        assert_eq!(a.turn_body.map(i32::abs), Some(900), "{m:?}");
        // The rest of the recovery leaves movement to the engine.
        for _ in 0..9 {
            let a = run_with_velocity(m, &mut mem, 50);
            assert_eq!((a.ahead, a.turn_body), (None, None), "{m:?}");
        }
        let a = run_with_velocity(m, &mut mem, 50);
        assert!(a.ahead != Some(-60), "{m:?} recovery ended");
    }
}

fn stuck_action(ahead: i32) -> Action {
    Action { ahead: Some(ahead), ..Action::default() }
}

fn still() -> Observation {
    Observation { velocity: 0, ..obs() }
}

#[test]
fn stuck_while_reversing_backs_off_forward() {
    let mut mem = ManeuverMemory::new(5);
    for _ in 0..3 {
        mem.recover(&still(), stuck_action(-100));
    }
    let a = mem.recover(&still(), stuck_action(-100));
    assert_eq!(a.ahead, Some(60));
}

#[test]
fn recovery_aborts_when_still_stuck() {
    let mut mem = ManeuverMemory::new(5);
    for _ in 0..3 {
        mem.recover(&still(), stuck_action(100));
    }
    let a = mem.recover(&still(), stuck_action(100));
    assert_eq!(a.ahead, Some(-60));
    // Still at velocity 0 on the next turn: recovery ends, the maneuver's
    // own movement returns, and detection starts over.
    let a = mem.recover(&still(), stuck_action(100));
    assert_eq!(a.ahead, Some(100));
    assert_eq!(mem.recovery_left, 0);
}

#[test]
fn moving_robot_is_not_stuck() {
    let mut mem = ManeuverMemory::new(5);
    for _ in 0..50 {
        let a = run_with_velocity(Maneuver::Patrol, &mut mem, 80);
        assert_ne!(a.ahead, Some(-60));
    }
    assert_eq!(mem.recovery_left, 0);
}

#[test]
fn defensive_is_exempt_from_recovery() {
    let mut mem = ManeuverMemory::new(5);
    for _ in 0..30 {
        let a = run_with_velocity(Maneuver::Defensive, &mut mem, 0);
        assert_ne!(a.ahead, Some(-60));
    }
}

#[test]
fn wall_guard_clips_ahead_near_wall() {
    use robocode::strategy::maneuver::memory::wall_guard;
    // Heading east (900), 100 px from the right wall of an 800 px field.
    let o = Observation { x: 700, heading: Angle(900), ..obs() };
    let a = wall_guard(&o, stuck_action(200));
    assert_eq!(a.ahead, Some(70)); // 800 - 30 - 700
    // Driving backward clips against the left side instead: plenty of room.
    assert_eq!(wall_guard(&o, stuck_action(-200)).ahead, Some(-200));
    // Already past the margin: a command into the wall becomes zero.
    let o = Observation { x: 790, ..o };
    assert_eq!(wall_guard(&o, stuck_action(50)).ahead, Some(0));
    // No command, no change.
    assert_eq!(wall_guard(&o, Action::default()).ahead, None);
}

#[test]
fn recovery_backs_off_away_from_wall() {
    // Heading east, close to the right wall, stuck while driving backward:
    // the opposite direction (forward) would hit the wall, so it backs off
    // backward by the room it has, and never emits Some(0).
    let o = Observation { x: 745, velocity: 0, heading: Angle(900), ..obs() };
    let mut mem = ManeuverMemory::new(5);
    for _ in 0..3 {
        mem.recover(&o, stuck_action(100));
    }
    let a = mem.recover(&o, stuck_action(100));
    // Stuck forward: back off backward (-60 is within the room behind).
    assert_eq!(a.ahead, Some(-60));
    // Stuck while reversing near the wall behind it: goes forward instead.
    let o = Observation { x: 45, velocity: 0, heading: Angle(900), ..obs() };
    let mut mem = ManeuverMemory::new(5);
    for _ in 0..3 {
        mem.recover(&o, stuck_action(-100));
    }
    let a = mem.recover(&o, stuck_action(-100));
    assert_eq!(a.ahead, Some(60));
    // Wall right behind and a short way ahead: only the turn remains.
    let o = Observation { x: 35, velocity: 0, heading: Angle(900), ..obs() };
    let mut mem = ManeuverMemory::new(5);
    let narrow = Observation { field_width: 70, ..o };
    for _ in 0..3 {
        mem.recover(&narrow, stuck_action(-100));
    }
    let a = mem.recover(&narrow, stuck_action(-100));
    assert_ne!(a.ahead, Some(0));
    assert!(a.turn_body.is_some());
}

#[test]
fn defensive_respects_the_wall_guard() {
    // Heading up (0), 25 px above the bottom wall, target close on the
    // right: Defensive backs away (-50), which points at the bottom wall.
    let o = Observation {
        y: 575,
        velocity: 0,
        ..with_target(900, 200, 50_000, 0, 0)
    };
    let mut mem = ManeuverMemory::new(5);
    let raw = moves::defensive(&o, &mut mem, &Risk::Normal.params());
    assert_eq!(raw.ahead, Some(-50));
    assert_eq!(raw.turn_body, Some(0));
    let a = Maneuver::Defensive.run(&o, &mut mem, &Risk::Normal.params());
    assert_eq!(a.ahead, Some(0), "backed into the bottom wall");
}

/// Unit direction of travel (x, y) for an Action from `o`, with y down.
fn travel(o: &Observation, a: &Action) -> (f64, f64) {
    let h = (o.heading.0 + a.turn_body.unwrap_or(0)) as f64;
    let rad = h / 10.0 * std::f64::consts::PI / 180.0;
    let s = a.ahead.unwrap_or(0).signum() as f64;
    (rad.sin() * s, -rad.cos() * s)
}

#[test]
fn evade_coin_sides_travel_in_opposite_directions() {
    let o = with_target(0, 300, 50_000, 0, 0);
    let mut by_dir = std::collections::HashMap::new();
    for seed in 1..50 {
        let mut mem = ManeuverMemory::new(seed);
        let a = moves::evade(&o, &mut mem, &Risk::Normal.params());
        by_dir.entry(mem.evade_dir).or_insert(travel(&o, &a));
    }
    let (p, n) = (by_dir[&1], by_dir[&-1]);
    let dot = p.0 * n.0 + p.1 * n.1;
    assert!(dot < -0.99, "both coin sides travel the same way: {p:?} {n:?}");
}

#[test]
fn evade_near_a_wall_takes_the_side_with_room() {
    // 40 px below the top wall, heading east, target due east: the side
    // heading north has 10 px of room, so every coin result goes south.
    let o = Observation {
        y: 40,
        heading: Angle(900),
        ..with_target(900, 300, 50_000, 0, 0)
    };
    for seed in 1..50 {
        let mut mem = ManeuverMemory::new(seed);
        let a = Maneuver::Evade.run(&o, &mut mem, &Risk::Normal.params());
        let (_, dy) = travel(&o, &a);
        assert!(dy > 0.99, "seed {seed} evaded toward the wall");
        assert!(a.ahead.is_some_and(|x| x != 0), "seed {seed} idles");
    }
    // The same 40 px above the bottom wall: every coin result goes north.
    let o = Observation { y: 560, ..o };
    for seed in 1..50 {
        let mut mem = ManeuverMemory::new(seed);
        let a = Maneuver::Evade.run(&o, &mut mem, &Risk::Normal.params());
        let (_, dy) = travel(&o, &a);
        assert!(dy < -0.99, "seed {seed} evaded toward the wall");
        assert!(a.ahead.is_some_and(|x| x != 0), "seed {seed} idles");
    }
}
