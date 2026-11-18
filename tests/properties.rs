//! Property tests: random battles must keep the engine's invariants on every
//! turn. A failing case is reported with the setup that caused it (quickcheck
//! prints the input), so it can be replayed as a regular test.

use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
use robocode::physics::BattleConfig;
use robocode::runner::Battle;
use robocode::terrain::Terrain;
use robocode::{Battlefield, Point, Ratio, Rect, recording, roster};

/// A random battle: 2–4 built-in robots on a 400–1000 px arena, with or
/// without the urban map, for 1–300 turns.
#[derive(Clone, Debug)]
struct Setup {
    robots: Vec<&'static str>,
    width: f64,
    height: f64,
    urban: bool,
    turns: u64,
}

impl Arbitrary for Setup {
    fn arbitrary(g: &mut Gen) -> Self {
        let n = 2 + usize::arbitrary(g) % 3;
        let robots = (0..n)
            .map(|_| {
                roster::BUILT_IN[usize::arbitrary(g) % roster::BUILT_IN.len()]
            })
            .collect();
        let side = |g: &mut Gen| 400.0 + (u16::arbitrary(g) % 601) as f64;
        Setup {
            robots,
            width: side(g),
            height: side(g),
            urban: bool::arbitrary(g),
            turns: 1 + u64::arbitrary(g) % 300,
        }
    }
}

fn build(s: &Setup) -> Battle {
    let robots = s
        .robots
        .iter()
        .map(|name| roster::built_in(name).expect("built-in robot"))
        .collect::<Vec<_>>();
    let config = BattleConfig {
        battlefield: Battlefield::new(s.width, s.height),
        ..BattleConfig::default()
    };
    let spawns = roster::spawns(s.robots.len(), s.width, s.height);
    let mut battle = Battle::new(config, &spawns, robots);
    if s.urban {
        battle.set_terrain(Terrain::urban(s.width, s.height));
    }
    battle
}

/// True if `p` is strictly inside `r` (on an edge doesn't count).
fn strictly_inside(p: Point, r: &Rect) -> bool {
    p.x > r.min.x && p.x < r.max.x && p.y > r.min.y && p.y < r.max.y
}

/// Plays the battle turn by turn and checks the engine's invariants after
/// every turn. Returns the first violation.
fn check_invariants(s: &Setup) -> Result<(), String> {
    let mut battle = build(s);
    let (w, h) = (s.width, s.height);
    // Where each robot was when it was first seen dead. A robot can move and
    // be killed in the same turn, so the check starts from that turn.
    let mut dead_at: Vec<Option<Point>> = vec![None; s.robots.len()];

    for turn in 1..=s.turns {
        battle.step();
        let state = battle.state();
        let buildings = &state.terrain.buildings;

        for r in &state.robots {
            let who = format!("turn {turn}, robot {}", r.id);
            for (name, v) in [
                ("x", r.position.x),
                ("y", r.position.y),
                ("energy", r.energy),
                ("gun heat", r.gun_heat),
                ("velocity", r.velocity),
                ("heading", r.heading),
            ] {
                if !v.is_finite() {
                    return Err(format!("{who}: {name} is not finite ({v})"));
                }
            }
            if r.gun_heat < 0.0 {
                return Err(format!(
                    "{who}: gun heat is negative ({})",
                    r.gun_heat
                ));
            }
            if r.alive {
                if r.energy <= 0.0 {
                    return Err(format!(
                        "{who}: alive with energy {}",
                        r.energy
                    ));
                }
                // Robots never leave the arena (walls clamp them).
                let eps = 1e-6;
                if r.position.x < -eps
                    || r.position.x > w + eps
                    || r.position.y < -eps
                    || r.position.y > h + eps
                {
                    return Err(format!(
                        "{who}: outside the arena at {:?}",
                        r.position
                    ));
                }
                if buildings.iter().any(|b| strictly_inside(r.position, b)) {
                    return Err(format!(
                        "{who}: inside a building at {:?}",
                        r.position
                    ));
                }
            } else {
                // The dead don't move.
                match dead_at[r.id] {
                    Some(p) if p != r.position => {
                        return Err(format!(
                            "{who}: a dead robot moved from {p:?}"
                        ));
                    }
                    Some(_) => {}
                    None => dead_at[r.id] = Some(r.position),
                }
            }
        }

        for b in &state.bullets {
            if !b.position.x.is_finite() || !b.position.y.is_finite() {
                return Err(format!(
                    "turn {turn}: bullet {} has a non-finite position",
                    b.id
                ));
            }
            if buildings.iter().any(|bl| strictly_inside(b.position, bl)) {
                return Err(format!(
                    "turn {turn}: bullet {} is inside a building",
                    b.id
                ));
            }
        }
    }
    Ok(())
}

fn to_result(outcome: Result<(), String>) -> TestResult {
    match outcome {
        Ok(()) => TestResult::passed(),
        Err(e) => TestResult::error(e),
    }
}

fn invariants_hold(s: Setup) -> TestResult {
    to_result(check_invariants(&s))
}

#[test]
fn every_turn_keeps_the_engine_invariants() {
    QuickCheck::new().quickcheck(invariants_hold as fn(Setup) -> TestResult);
}

#[test]
fn the_same_setup_always_records_the_same_battle() {
    fn same_twice(s: Setup) -> TestResult {
        let a = recording::record(build(&s), s.turns);
        let b = recording::record(build(&s), s.turns);
        to_result(if a == b {
            Ok(())
        } else {
            Err("two runs of the same setup differ".to_string())
        })
    }
    QuickCheck::new()
        .tests(25)
        .quickcheck(same_twice as fn(Setup) -> TestResult);
}

/// Where a segment first touches a rectangle, the touching point is on or
/// inside the rectangle (up to rounding), and the fraction is in [0, 1].
#[test]
fn a_reported_contact_lies_on_the_rectangle() {
    #[derive(Clone, Debug)]
    struct Case {
        rect: (f64, f64, f64, f64),
        from: (f64, f64),
        to: (f64, f64),
    }
    impl Arbitrary for Case {
        fn arbitrary(g: &mut Gen) -> Self {
            let coord = |g: &mut Gen| (i16::arbitrary(g) % 1000) as f64;
            let size = |g: &mut Gen| 1.0 + (u8::arbitrary(g) % 200) as f64;
            Case {
                rect: (coord(g), coord(g), size(g), size(g)),
                from: (coord(g), coord(g)),
                to: (coord(g), coord(g)),
            }
        }
    }
    fn prop(c: Case) -> TestResult {
        let (x, y, w, h) = c.rect;
        let r = Rect::new(x, y, w, h);
        let from = Point::new(c.from.0, c.from.1);
        let to = Point::new(c.to.0, c.to.1);
        match r.first_contact(from, to) {
            None => TestResult::passed(),
            Some(t) => {
                if !(0.0..=1.0).contains(&t) {
                    return TestResult::error(format!(
                        "fraction {t} is outside [0, 1]"
                    ));
                }
                let p = Point::new(
                    from.x + (to.x - from.x) * t,
                    from.y + (to.y - from.y) * t,
                );
                let eps = 1e-6;
                let on = p.x >= r.min.x - eps
                    && p.x <= r.max.x + eps
                    && p.y >= r.min.y - eps
                    && p.y <= r.max.y + eps;
                if on {
                    TestResult::passed()
                } else {
                    TestResult::error(format!("contact {p:?} misses {r:?}"))
                }
            }
        }
    }
    QuickCheck::new().tests(2000).quickcheck(prop as fn(Case) -> TestResult);
}

// ---------------------------------------------------------------------------
// Radar beam: what the drawing shows against what the scan sees.

use robocode::line_of_sight;
use robocode::physics::radar::radar_beam;
use robocode::physics::{BattleState, RobotState, ScannedRobot};
use robocode::rules::{RADAR_SCAN_RADIUS, ROBOT_RADIUS};

/// A uniform number in [0, 1] (0 and 1 themselves come up now and then).
fn unit(g: &mut Gen) -> f64 {
    u32::arbitrary(g) as f64 / u32::MAX as f64
}

fn between(g: &mut Gen, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * unit(g)
}

/// Distance from `p` to the rectangle (0 inside it).
fn distance_to_rect(p: Point, r: &Rect) -> f64 {
    let dx = (r.min.x - p.x).max(0.0).max(p.x - r.max.x);
    let dy = (r.min.y - p.y).max(0.0).max(p.y - r.max.y);
    (dx * dx + dy * dy).sqrt()
}

/// One radar turn: two still robots, buildings, and each robot's radar
/// heading before the turn and how far it sweeps.
#[derive(Clone, Debug)]
struct ScanCase {
    width: f64,
    height: f64,
    buildings: Vec<Rect>,
    robots: [Point; 2],
    /// (radar heading before the turn, sweep), per robot.
    radars: [(f64, f64); 2],
}

impl ScanCase {
    fn field(&self) -> Battlefield {
        Battlefield::new(self.width, self.height)
    }

    /// Robots inside the field, clear of every building by more than a
    /// robot's radius, and clear of each other.
    fn valid(&self) -> bool {
        let r = ROBOT_RADIUS;
        let in_field = |p: &Point| {
            p.x >= r
                && p.x <= self.width - r
                && p.y >= r
                && p.y <= self.height - r
        };
        let clear = |p: &Point| {
            self.buildings.iter().all(|b| distance_to_rect(*p, b) > r + 0.01)
        };
        let [a, b] = self.robots;
        self.robots.iter().all(|p| in_field(p) && clear(p))
            && ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
                > 2.0 * r + 1.0
    }
}

fn random_sweep(g: &mut Gen) -> f64 {
    if u8::arbitrary(g) % 4 == 0 { 0.0 } else { between(g, -45.0, 45.0) }
}

fn random_building(g: &mut Gen, w: f64, h: f64) -> Rect {
    let (bw, bh) = (between(g, 10.0, 200.0), between(g, 10.0, 200.0));
    Rect::new(between(g, 0.0, w - bw), between(g, 0.0, h - bh), bw, bh)
}

impl Arbitrary for ScanCase {
    fn arbitrary(g: &mut Gen) -> Self {
        let r = ROBOT_RADIUS;
        let mut last = None;
        for _ in 0..200 {
            let (width, height) = if bool::arbitrary(g) {
                Ratio::ALL[usize::arbitrary(g) % 4].size()
            } else {
                (between(g, 400.0, 1400.0), between(g, 400.0, 1400.0))
            };
            let a = Point::new(
                between(g, r, width - r),
                between(g, r, height - r),
            );
            let a_radar = (between(g, 0.0, 360.0), random_sweep(g));
            // The target: mostly near an edge of the swept arc, where its body
            // overlaps the arc while its centre is outside the swept angle.
            let distance = between(g, 2.0 * r + 1.0, RADAR_SCAN_RADIUS + 5.0);
            let half = (r / distance).asin().to_degrees();
            let (start, sweep) = a_radar;
            let bearing = match u8::arbitrary(g) % 4 {
                0 => start + between(g, -half - 1.0, half + 1.0),
                1 => start + sweep + between(g, -half - 1.0, half + 1.0),
                2 => start + between(g, -10.0, 10.0) + sweep * unit(g),
                _ => between(g, 0.0, 360.0),
            };
            let b = a.translate(bearing, distance);
            let b_radar = (between(g, 0.0, 360.0), random_sweep(g));

            let mut buildings: Vec<Rect> = (0..1 + usize::arbitrary(g) % 6)
                .map(|_| random_building(g, width, height))
                .collect();
            // Often one building has a corner just off the line of sight, so
            // the scan is decided by a few pixels.
            if bool::arbitrary(g) {
                let t = between(g, 0.2, 0.8);
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let len = (dx * dx + dy * dy).sqrt();
                let off = between(g, -3.0, 3.0);
                let corner = Point::new(
                    a.x + dx * t - dy / len * off,
                    a.y + dy * t + dx / len * off,
                );
                let (bw, bh) =
                    (between(g, 10.0, 120.0), between(g, 10.0, 120.0));
                let x =
                    if bool::arbitrary(g) { corner.x } else { corner.x - bw };
                let y =
                    if bool::arbitrary(g) { corner.y } else { corner.y - bh };
                buildings.push(Rect::new(x, y, bw, bh));
            }
            let case = ScanCase {
                width,
                height,
                buildings,
                robots: [a, b],
                radars: [a_radar, b_radar],
            };
            if case.valid() {
                return case;
            }
            last = Some(case);
        }
        last.expect("at least one attempt")
    }
}

/// The engine's relative-angle normalisation, in (-180, 180].
fn relative(angle: f64) -> f64 {
    let mut a = angle % 360.0;
    if a <= -180.0 {
        a += 360.0;
    } else if a > 180.0 {
        a -= 360.0;
    }
    if a.abs() < robocode::rules::EPSILON { 0.0 } else { a }
}

/// Runs one turn and returns the battle afterwards and the scan events.
fn run_scan_turn(
    c: &ScanCase,
) -> Result<(BattleState, Vec<ScannedRobot>), String> {
    let robots = (0..2)
        .map(|i| {
            let mut robot = RobotState::new(i, c.robots[i]);
            robot.radar_heading = c.radars[i].0;
            robot.control.radar_turn_remaining = c.radars[i].1;
            robot
        })
        .collect();
    let config =
        BattleConfig { battlefield: c.field(), ..BattleConfig::default() };
    let mut battle = BattleState::new(config, robots);
    battle.terrain =
        Terrain { buildings: c.buildings.clone(), roads: Vec::new() };
    let scans = battle.step().scans;
    // The scan saw the robots where they stand now only if nothing moved.
    for (i, robot) in battle.robots.iter().enumerate() {
        if robot.position != c.robots[i] {
            return Err(format!("robot {i} moved to {:?}", robot.position));
        }
    }
    Ok((battle, scans))
}

/// The swept radar arc of `robot` this turn, as `radar_beam` takes it.
fn beam_of(robot: &RobotState, c: &ScanCase) -> Vec<Point> {
    let sweep = relative(robot.radar_heading - robot.last_radar_heading);
    radar_beam(
        robot.position,
        robot.last_radar_heading,
        sweep,
        &c.buildings,
        &c.field(),
    )
}

/// Even-odd point-in-polygon test.
fn polygon_contains(poly: &[Point], p: Point) -> bool {
    let mut inside = false;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if p.x < x {
                inside = !inside;
            }
        }
    }
    inside
}

fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0)
    };
    let (cx, cy) = (a.x + dx * t - p.x, a.y + dy * t - p.y);
    (cx * cx + cy * cy).sqrt()
}

fn distance_to_polygon(poly: &[Point], p: Point) -> f64 {
    (0..poly.len())
        .map(|i| distance_to_segment(p, poly[i], poly[(i + 1) % poly.len()]))
        .fold(f64::INFINITY, f64::min)
}

fn quickcheck_tests(default: u64) -> u64 {
    std::env::var("QUICKCHECK_TESTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// The drawing never lies: every robot the engine scans has its centre in
/// the scanner's beam polygon, or within 0.5 px of its edge.
#[test]
fn scanned_centres_lie_inside_the_beam() {
    fn prop(c: ScanCase) -> TestResult {
        if !c.valid() {
            return TestResult::discard();
        }
        let (battle, scans) = match run_scan_turn(&c) {
            Ok(v) => v,
            Err(e) => return TestResult::error(e),
        };
        for s in &scans {
            let scanner = &battle.robots[s.scanner];
            let centre = battle.robots[s.target].position;
            let beam = beam_of(scanner, &c);
            if !polygon_contains(&beam, centre) {
                let off = distance_to_polygon(&beam, centre);
                if off.is_nan() || off > 0.5 {
                    return TestResult::error(format!(
                        "robot {} scanned robot {} at {centre:?}, {off} px outside \
                         its beam {beam:?} in {c:?}",
                        s.scanner, s.target
                    ));
                }
            }
        }
        TestResult::passed()
    }
    let n = quickcheck_tests(1000);
    QuickCheck::new()
        .tests(n)
        .max_tests(n * 10)
        .quickcheck(prop as fn(ScanCase) -> TestResult);
}

/// The engine's arc test: the interval `rel ± half_width` overlaps 0..sweep.
fn arc_overlaps(sweep: f64, rel: f64, half_width: f64) -> bool {
    if half_width >= 180.0 {
        return true;
    }
    let (lo, hi) = if sweep >= 0.0 { (0.0, sweep) } else { (sweep, 0.0) };
    [rel - 360.0, rel, rel + 360.0]
        .iter()
        .any(|r| r + half_width >= lo && r - half_width <= hi)
}

/// A target in range whose body overlaps the swept arc is scanned exactly
/// when the shared line-of-sight function says the centres see each other.
#[test]
fn scan_line_of_sight_matches_shared_function() {
    fn prop(c: ScanCase) -> TestResult {
        if !c.valid() {
            return TestResult::discard();
        }
        let (battle, scans) = match run_scan_turn(&c) {
            Ok(v) => v,
            Err(e) => return TestResult::error(e),
        };
        for a in &battle.robots {
            for b in &battle.robots {
                if a.id == b.id {
                    continue;
                }
                let (dx, dy) =
                    (b.position.x - a.position.x, b.position.y - a.position.y);
                let distance = (dx * dx + dy * dy).sqrt();
                let bearing = dx.atan2(-dy).to_degrees();
                let half_width =
                    (b.radius / distance).min(1.0).asin().to_degrees();
                let sweep = relative(a.radar_heading - a.last_radar_heading);
                let rel = relative(bearing - a.last_radar_heading);
                if distance > RADAR_SCAN_RADIUS
                    || !arc_overlaps(sweep, rel, half_width)
                {
                    continue;
                }
                let scanned = scans
                    .iter()
                    .any(|s| s.scanner == a.id && s.target == b.id);
                let sight =
                    line_of_sight(a.position, b.position, &c.buildings);
                if scanned != sight {
                    return TestResult::error(format!(
                        "robot {} -> {}: scanned {scanned}, line of sight {sight}",
                        a.id, b.id
                    ));
                }
            }
        }
        TestResult::passed()
    }
    let n = quickcheck_tests(1000);
    QuickCheck::new()
        .tests(n)
        .max_tests(n * 10)
        .quickcheck(prop as fn(ScanCase) -> TestResult);
}

// ---- maneuvers never fire without a fresh, affordable target ----

use robocode::strategy::maneuver::memory::ManeuverMemory;
use robocode::strategy::maneuver::{Maneuver, Risk};
use robocode::strategy::{Observation, TargetInfo};
use robocode::units::{Angle, Energy, GunHeat};

#[derive(Clone, Debug)]
struct ManeuverCase(Observation);

impl Arbitrary for ManeuverCase {
    fn arbitrary(g: &mut Gen) -> Self {
        let angle = |g: &mut Gen| Angle((u16::arbitrary(g) % 3600) as i32);
        let w = 400 + (u16::arbitrary(g) % 4601) as i32;
        let h = 400 + (u16::arbitrary(g) % 4601) as i32;
        let target = bool::arbitrary(g).then(|| TargetInfo {
            bearing: angle(g),
            distance: (u16::arbitrary(g) % 6000) as i32,
            energy: Energy(u32::arbitrary(g) % 200_001),
            heading: angle(g),
            velocity: (i16::arbitrary(g) % 81) as i32,
            age: u32::arbitrary(g) % 41,
        });
        ManeuverCase(Observation {
            turn: u32::arbitrary(g) % 10_000,
            energy: Energy(u32::arbitrary(g) % 200_001),
            gun_heat: GunHeat(0),
            heading: angle(g),
            gun_heading: angle(g),
            radar_heading: angle(g),
            velocity: (i16::arbitrary(g) % 81) as i32,
            x: (u16::arbitrary(g) as i32) % (w + 1),
            y: (u16::arbitrary(g) as i32) % (h + 1),
            field_width: w,
            field_height: h,
            target,
        })
    }
}

#[test]
fn no_maneuver_fires_without_a_fresh_affordable_target() {
    fn prop(case: ManeuverCase) -> TestResult {
        let obs = case.0;
        for m in [
            Maneuver::Search,
            Maneuver::Patrol,
            Maneuver::Defensive,
            Maneuver::Evade,
            Maneuver::Attack,
        ] {
            for risk in [Risk::Cautious, Risk::Normal, Risk::Aggressive] {
                for turns in [0, 5] {
                    let mut mem = ManeuverMemory::new(42);
                    mem.turns_in_maneuver = turns;
                    let a = m.run(&obs, &mut mem, &risk.params());
                    if let Some(p) = a.fire {
                        let fresh = obs.target.is_some_and(|t| t.age == 0);
                        if !fresh || p.0 >= obs.energy.0 {
                            return TestResult::failed();
                        }
                    }
                    if !matches!(a.turn_radar, Some(r) if r != 0) {
                        return TestResult::failed();
                    }
                }
            }
        }
        TestResult::passed()
    }
    QuickCheck::new()
        .tests(2000)
        .quickcheck(prop as fn(ManeuverCase) -> TestResult);
}
