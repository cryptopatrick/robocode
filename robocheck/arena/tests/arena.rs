use robocheck_arena::{ArenaRobot, Surroundings, clearance, is_ready};
use robocode::Battlefield;
use robocode::api::Robot;
use robocode::physics::BattleConfig;
use robocode::recording::record_with_summary;
use robocode::robots::safe_shooter::{self, SafeShooter};
use robocode::roster;
use robocode::runner::{Battle, boxed};
use robocode::strategy::{Observation, TargetInfo};
use robocode::terrain::Terrain;
use robocode::units::{Angle, Energy, GunHeat};

const W: f64 = 800.0;
const H: f64 = 600.0;

#[test]
fn open_field_east() {
    let s = clearance(&Terrain::default(), W, H, 400.0, 300.0, 90.0);
    assert_eq!(
        s,
        Surroundings { clearance_ahead: 382, clearance_behind: 382 }
    );
}

#[test]
fn open_field_north() {
    // y grows downward: heading 0 points up, toward y = 0.
    let s = clearance(&Terrain::default(), W, H, 400.0, 300.0, 0.0);
    assert_eq!(
        s,
        Surroundings { clearance_ahead: 282, clearance_behind: 282 }
    );
}

#[test]
fn building_in_front() {
    // The urban map's building at x 200..320, y 60..140; grown by the 18 px radius it starts at x = 182.
    let s = clearance(&Terrain::urban(W, H), W, H, 170.0, 100.0, 90.0);
    assert_eq!(s.clearance_ahead, 12);
}

#[test]
fn rounds_down() {
    // The east wall, inset by the radius, is at x = 782: 81.9 px away.
    let s = clearance(&Terrain::default(), W, H, 700.1, 300.0, 90.0);
    assert_eq!(s.clearance_ahead, 81);
}

fn safe_shooter_decide(
    obs: &Observation,
    _env: &Surroundings,
) -> robocode::strategy::Action {
    safe_shooter::decide(obs)
}

fn battle(robots: Vec<Box<dyn Robot>>) -> Battle {
    let config = BattleConfig {
        battlefield: Battlefield::new(W, H),
        ..BattleConfig::default()
    };
    Battle::new(config, &roster::spawns(2, W, H), robots)
}

#[test]
fn arena_robot_matches_strategy_robot() {
    let plain = record_with_summary(
        battle(vec![boxed(SafeShooter), roster::built_in("Circler").unwrap()]),
        2000,
    );
    let robot = ArenaRobot::new(
        "SafeShooter",
        safe_shooter_decide,
        Terrain::default(),
        W,
        H,
    );
    let wrapped = record_with_summary(
        battle(vec![Box::new(robot), roster::built_in("Circler").unwrap()]),
        2000,
    );
    assert_eq!(plain.json, wrapped.json);
}

#[test]
fn decision_log_has_every_turn_and_full_observation() {
    let robot = ArenaRobot::new(
        "SafeShooter",
        safe_shooter_decide,
        Terrain::urban(W, H),
        W,
        H,
    );
    let log = robot.log();
    let mut b = battle(vec![
        Box::new(robot),
        roster::built_in("SittingDuck").unwrap(),
    ]);
    b.set_terrain(Terrain::urban(W, H));
    let rec = record_with_summary(b, 300);
    let records = log.borrow();
    assert!(!records.is_empty());
    // One record per turn the robot acted, in order, with no gaps.
    for pair in records.windows(2) {
        assert_eq!(pair[1].turn, pair[0].turn + 1);
    }
    assert!(records.len() as u64 <= rec.turns);
    let line = records.last().unwrap().to_json_line();
    for key in [
        "\"turn\"",
        "\"energy\"",
        "\"gun_heat\"",
        "\"heading\"",
        "\"gun_heading\"",
        "\"radar_heading\"",
        "\"velocity\"",
        "\"x\"",
        "\"y\"",
        "\"field_width\"",
        "\"field_height\"",
        "\"target\"",
        "\"clearance_ahead\"",
        "\"clearance_behind\"",
        "\"fire\"",
        "\"ahead\"",
        "\"turn_body\"",
        "\"turn_gun\"",
        "\"turn_radar\"",
    ] {
        assert!(line.contains(key), "{key} missing from {line}");
    }
    assert!(!line.contains('\n'));
}

/// A small deterministic generator of well-formed observations.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % n
    }
}

fn observation(r: &mut Lcg) -> Observation {
    let target = (r.next(4) != 0).then(|| TargetInfo {
        bearing: Angle(r.next(3600) as i32),
        distance: r.next(900) as i32,
        energy: Energy(r.next(120_000) as u32),
        heading: Angle(r.next(3600) as i32),
        velocity: r.next(161) as i32 - 80,
        age: r.next(3) as u32,
    });
    let gun_heading = match (target, r.next(2)) {
        // Often aim near the target, so `ready` is true for a fair share of cases.
        (Some(t), 0) => {
            Angle(((t.bearing.0 + r.next(61) as i32 - 30) + 3600) % 3600)
        }
        _ => Angle(r.next(3600) as i32),
    };
    Observation {
        turn: r.next(5000) as u32,
        energy: Energy(r.next(4000) as u32),
        gun_heat: GunHeat(if r.next(2) == 0 {
            0
        } else {
            r.next(1600) as u32
        }),
        heading: Angle(r.next(3600) as i32),
        gun_heading,
        radar_heading: Angle(r.next(3600) as i32),
        velocity: r.next(161) as i32 - 80,
        x: r.next(800) as i32,
        y: r.next(600) as i32,
        field_width: 800,
        field_height: 600,
        target,
    }
}

#[test]
fn is_ready_matches_safe_shooter_decision() {
    let mut r = Lcg(7);
    let mut ready = 0;
    for _ in 0..1000 {
        let obs = observation(&mut r);
        assert_eq!(
            is_ready(&obs),
            safe_shooter::decide(&obs).fire.is_some(),
            "{obs:?}"
        );
        ready += is_ready(&obs) as u32;
    }
    assert!(
        ready > 20,
        "only {ready} ready cases: the generator doesn't exercise firing"
    );
}
