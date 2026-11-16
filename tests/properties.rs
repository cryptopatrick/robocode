//! Property tests: random battles must keep the engine's invariants on every
//! turn. A failing case is reported with the setup that caused it (quickcheck
//! prints the input), so it can be replayed as a regular test.

use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
use robocode::physics::BattleConfig;
use robocode::runner::Battle;
use robocode::terrain::Terrain;
use robocode::{Battlefield, Point, Rect, recording, roster};

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
