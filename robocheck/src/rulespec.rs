//! The spec language the AI writes in. The AI never writes Verus text: it
//! states each rule as a small JSON tree, which RoboCheck validates
//! ([`validate`]), turns into one Verus `ensures` clause ([`render_verus`]),
//! and executes to find broken turns in a battle ([`eval`]).
//!
//! Units are robocode's fixed-point units: energy, gun heat and power in
//! thousandths; angles in decidegrees (0..3599, 0 = north, clockwise);
//! distances in whole pixels; velocity in decipixels per turn.

use robocheck_arena::{Surroundings, is_ready};
use robocode::robots::safe_shooter::is_aimed;
use robocode::strategy::{Action, Observation};
use serde::{Deserialize, Serialize};

/// Largest tree depth (conditions and terms) of one clause.
pub const MAX_DEPTH: usize = 6;
/// Most nodes (conditions and terms) in one clause.
pub const MAX_NODES: usize = 40;
/// Largest absolute value of a number in a clause.
pub const MAX_NUM: i64 = 1_000_000;
/// Most `mul` nodes in one clause, so evaluation stays well inside `i128`.
pub const MAX_MULS: usize = 2;

/// A rule as the visitor (or the starter set) states it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub text: String,
}

/// When a clause applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    /// Whenever the robot fires (the shot's power is `shot_power`).
    Fires,
    /// Whenever it drives forward (the distance is `move_distance`).
    DrivesForward,
    /// Whenever it drives backward (`move_distance` is negative).
    DrivesBackward,
    /// Every turn.
    Always,
}

/// One clause of a rule's spec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clause {
    pub when: Trigger,
    pub require: Cond,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Cond {
    All(Vec<Cond>),
    Any(Vec<Cond>),
    Not(Box<Cond>),
    Cmp(Cmp),
    Pred(Pred),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cmp {
    pub left: Term,
    pub op: Op,
    pub right: Term,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    #[serde(rename = "<")]
    Lt,
    #[serde(rename = "<=")]
    Le,
    #[serde(rename = "==")]
    Eq,
    #[serde(rename = "!=")]
    Ne,
    #[serde(rename = ">=")]
    Ge,
    #[serde(rename = ">")]
    Gt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pred {
    /// A target was scanned this turn.
    TargetSeenNow,
    /// The gun can fire (heat 0).
    GunCool,
    /// The gun points at the target within 3.0°.
    Aimed,
    /// SafeShooter's `ready`: scanned now, cool, aimed, energy above 1.0.
    Ready,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Term {
    Num(i64),
    Obs(ObsField),
    Target(TargetField),
    /// Must be `true`; only under `fires`.
    ShotPower(bool),
    /// Must be `true`; only under `drives_forward` / `drives_backward`.
    MoveDistance(bool),
    Add(Box<(Term, Term)>),
    Sub(Box<(Term, Term)>),
    Mul(Box<(Term, Term)>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObsField {
    Turn,
    Energy,
    GunHeat,
    Heading,
    GunHeading,
    RadarHeading,
    Velocity,
    X,
    Y,
    FieldWidth,
    FieldHeight,
    ClearanceAhead,
    ClearanceBehind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetField {
    Bearing,
    Distance,
    Energy,
    Heading,
    Velocity,
    Age,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StarterFile {
    rules: Vec<StarterRule>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StarterRule {
    id: String,
    text: String,
    reference: Vec<Clause>,
}

const STARTER_JSON: &str = include_str!("../starter_rules.json");

fn starter_file() -> StarterFile {
    serde_json::from_str(STARTER_JSON).expect("robocheck/starter_rules.json is valid")
}

/// The three starter rules, S1–S3.
pub fn starter_rules() -> Vec<Rule> {
    starter_file().rules.into_iter().map(|r| Rule { id: r.id, text: r.text }).collect()
}

/// The hand formalisation of a starter rule (empty for any other id).
pub fn reference_clauses(id: &str) -> Vec<Clause> {
    starter_file().rules.into_iter().find(|r| r.id == id).map(|r| r.reference).unwrap_or_default()
}

// ---------- validation ----------

struct Stats {
    depth: usize,
    nodes: usize,
    muls: usize,
}

fn check_term(t: &Term, when: Trigger, depth: usize, s: &mut Stats) -> Result<(), String> {
    s.nodes += 1;
    s.depth = s.depth.max(depth);
    match t {
        Term::Num(n) if n.abs() > MAX_NUM => Err(format!("number {n} is outside ±{MAX_NUM}")),
        Term::Num(_) | Term::Obs(_) | Term::Target(_) => Ok(()),
        Term::ShotPower(b) => {
            if !b {
                Err("`shot_power` must be true".into())
            } else if when != Trigger::Fires {
                Err("`shot_power` is only defined when the clause applies to firing (`when: fires`)".into())
            } else {
                Ok(())
            }
        }
        Term::MoveDistance(b) => {
            if !b {
                Err("`move_distance` must be true".into())
            } else if !matches!(when, Trigger::DrivesForward | Trigger::DrivesBackward) {
                Err("`move_distance` is only defined when the clause applies to driving".into())
            } else {
                Ok(())
            }
        }
        Term::Add(p) | Term::Sub(p) | Term::Mul(p) => {
            if matches!(t, Term::Mul(_)) {
                s.muls += 1;
            }
            check_term(&p.0, when, depth + 1, s)?;
            check_term(&p.1, when, depth + 1, s)
        }
    }
}

fn check_cond(c: &Cond, when: Trigger, depth: usize, s: &mut Stats) -> Result<(), String> {
    s.nodes += 1;
    s.depth = s.depth.max(depth);
    match c {
        Cond::All(cs) | Cond::Any(cs) => cs.iter().try_for_each(|c| check_cond(c, when, depth + 1, s)),
        Cond::Not(c) => check_cond(c, when, depth + 1, s),
        Cond::Cmp(cmp) => {
            check_term(&cmp.left, when, depth + 1, s)?;
            check_term(&cmp.right, when, depth + 1, s)
        }
        Cond::Pred(_) => Ok(()),
    }
}

/// Checks a clause against the language's limits and scoping rules.
pub fn validate(c: &Clause) -> Result<(), String> {
    let mut s = Stats { depth: 0, nodes: 0, muls: 0 };
    check_cond(&c.require, c.when, 1, &mut s)?;
    if s.depth > MAX_DEPTH {
        return Err(format!("the clause is {} levels deep; at most {MAX_DEPTH}", s.depth));
    }
    if s.nodes > MAX_NODES {
        return Err(format!("the clause has {} nodes; at most {MAX_NODES}", s.nodes));
    }
    if s.muls > MAX_MULS {
        return Err(format!("the clause multiplies {} times; at most {MAX_MULS}", s.muls));
    }
    Ok(())
}

// ---------- Verus rendering ----------

fn uses_target(t: &Term) -> bool {
    match t {
        Term::Target(_) => true,
        Term::Add(p) | Term::Sub(p) | Term::Mul(p) => uses_target(&p.0) || uses_target(&p.1),
        _ => false,
    }
}

fn obs_verus(f: ObsField) -> &'static str {
    match f {
        ObsField::Turn => "(obs.turn as int)",
        ObsField::Energy => "(obs.energy.0 as int)",
        ObsField::GunHeat => "(obs.gun_heat.0 as int)",
        ObsField::Heading => "(obs.heading.0 as int)",
        ObsField::GunHeading => "(obs.gun_heading.0 as int)",
        ObsField::RadarHeading => "(obs.radar_heading.0 as int)",
        ObsField::Velocity => "(obs.velocity as int)",
        ObsField::X => "(obs.x as int)",
        ObsField::Y => "(obs.y as int)",
        ObsField::FieldWidth => "(obs.field_width as int)",
        ObsField::FieldHeight => "(obs.field_height as int)",
        ObsField::ClearanceAhead => "(around.clearance_ahead as int)",
        ObsField::ClearanceBehind => "(around.clearance_behind as int)",
    }
}

fn target_verus(f: TargetField) -> &'static str {
    match f {
        TargetField::Bearing => "(t.bearing.0 as int)",
        TargetField::Distance => "(t.distance as int)",
        TargetField::Energy => "(t.energy.0 as int)",
        TargetField::Heading => "(t.heading.0 as int)",
        TargetField::Velocity => "(t.velocity as int)",
        TargetField::Age => "(t.age as int)",
    }
}

fn term_verus(t: &Term) -> String {
    match t {
        Term::Num(n) if *n < 0 => format!("({n})"),
        Term::Num(n) => n.to_string(),
        Term::Obs(f) => obs_verus(*f).into(),
        Term::Target(f) => target_verus(*f).into(),
        Term::ShotPower(_) => "(p.0 as int)".into(),
        Term::MoveDistance(_) => "(a as int)".into(),
        Term::Add(p) => format!("({} + {})", term_verus(&p.0), term_verus(&p.1)),
        Term::Sub(p) => format!("({} - {})", term_verus(&p.0), term_verus(&p.1)),
        Term::Mul(p) => format!("({} * {})", term_verus(&p.0), term_verus(&p.1)),
    }
}

fn op_str(op: Op) -> &'static str {
    match op {
        Op::Lt => "<",
        Op::Le => "<=",
        Op::Eq => "==",
        Op::Ne => "!=",
        Op::Ge => ">=",
        Op::Gt => ">",
    }
}

/// Every condition renders as one atomic group: parenthesised, or a call.
fn cond_verus(c: &Cond) -> String {
    match c {
        Cond::All(cs) if cs.is_empty() => "true".into(),
        Cond::Any(cs) if cs.is_empty() => "false".into(),
        Cond::All(cs) => format!("({})", cs.iter().map(cond_verus).collect::<Vec<_>>().join(" && ")),
        Cond::Any(cs) => format!("({})", cs.iter().map(cond_verus).collect::<Vec<_>>().join(" || ")),
        Cond::Not(c) => format!("(!{})", cond_verus(c)),
        Cond::Cmp(cmp) => {
            let body = format!("{} {} {}", term_verus(&cmp.left), op_str(cmp.op), term_verus(&cmp.right));
            if uses_target(&cmp.left) || uses_target(&cmp.right) {
                format!("(obs.target matches Some(t) && {body})")
            } else {
                format!("({body})")
            }
        }
        Cond::Pred(Pred::TargetSeenNow) => "(obs.target matches Some(t) && t.age == 0)".into(),
        Cond::Pred(Pred::GunCool) => "(obs.gun_heat.0 == 0)".into(),
        Cond::Pred(Pred::Aimed) => "aimed(*obs)".into(),
        Cond::Pred(Pred::Ready) => "ready(*obs)".into(),
    }
}

/// The clause as one Verus `ensures` line (no newline, no trailing comma),
/// over `obs: &Observation`, `around: &Surroundings` and the result `r`.
pub fn render_verus(c: &Clause) -> String {
    let body = cond_verus(&c.require);
    match c.when {
        Trigger::Fires => format!("r.fire matches Some(p) ==> {body}"),
        Trigger::DrivesForward => format!("r.ahead matches Some(a) ==> (a > 0 ==> {body})"),
        Trigger::DrivesBackward => format!("r.ahead matches Some(a) ==> (a < 0 ==> {body})"),
        Trigger::Always => body,
    }
}

// ---------- execution ----------

struct Ctx<'a> {
    obs: &'a Observation,
    env: &'a Surroundings,
    shot: Option<i128>,
    moved: Option<i128>,
}

/// `None` when the term reads the target and there is none.
fn term_val(t: &Term, x: &Ctx) -> Option<i128> {
    let o = x.obs;
    Some(match t {
        Term::Num(n) => *n as i128,
        Term::Obs(f) => match f {
            ObsField::Turn => o.turn as i128,
            ObsField::Energy => o.energy.0 as i128,
            ObsField::GunHeat => o.gun_heat.0 as i128,
            ObsField::Heading => o.heading.0 as i128,
            ObsField::GunHeading => o.gun_heading.0 as i128,
            ObsField::RadarHeading => o.radar_heading.0 as i128,
            ObsField::Velocity => o.velocity as i128,
            ObsField::X => o.x as i128,
            ObsField::Y => o.y as i128,
            ObsField::FieldWidth => o.field_width as i128,
            ObsField::FieldHeight => o.field_height as i128,
            ObsField::ClearanceAhead => x.env.clearance_ahead as i128,
            ObsField::ClearanceBehind => x.env.clearance_behind as i128,
        },
        Term::Target(f) => {
            let t = o.target?;
            match f {
                TargetField::Bearing => t.bearing.0 as i128,
                TargetField::Distance => t.distance as i128,
                TargetField::Energy => t.energy.0 as i128,
                TargetField::Heading => t.heading.0 as i128,
                TargetField::Velocity => t.velocity as i128,
                TargetField::Age => t.age as i128,
            }
        }
        Term::ShotPower(_) => x.shot.expect("validated: shot_power only under fires"),
        Term::MoveDistance(_) => x.moved.expect("validated: move_distance only under driving"),
        Term::Add(p) => term_val(&p.0, x)? + term_val(&p.1, x)?,
        Term::Sub(p) => term_val(&p.0, x)? - term_val(&p.1, x)?,
        Term::Mul(p) => term_val(&p.0, x)?.saturating_mul(term_val(&p.1, x)?),
    })
}

fn cond_val(c: &Cond, x: &Ctx) -> bool {
    match c {
        Cond::All(cs) => cs.iter().all(|c| cond_val(c, x)),
        Cond::Any(cs) => cs.iter().any(|c| cond_val(c, x)),
        Cond::Not(c) => !cond_val(c, x),
        Cond::Cmp(cmp) => match (term_val(&cmp.left, x), term_val(&cmp.right, x)) {
            (Some(l), Some(r)) => match cmp.op {
                Op::Lt => l < r,
                Op::Le => l <= r,
                Op::Eq => l == r,
                Op::Ne => l != r,
                Op::Ge => l >= r,
                Op::Gt => l > r,
            },
            // As rendered for Verus: a comparison that reads the target is false without one.
            _ => false,
        },
        Cond::Pred(Pred::TargetSeenNow) => x.obs.target.is_some_and(|t| t.age == 0),
        Cond::Pred(Pred::GunCool) => x.obs.gun_heat.0 == 0,
        Cond::Pred(Pred::Aimed) => is_aimed(x.obs),
        Cond::Pred(Pred::Ready) => is_ready(x.obs),
    }
}

/// Whether one turn (what the robot saw and did) keeps the clause.
/// `c` must have passed [`validate`].
pub fn eval(c: &Clause, obs: &Observation, env: &Surroundings, act: &Action) -> bool {
    let mut x = Ctx { obs, env, shot: None, moved: None };
    match c.when {
        Trigger::Fires => match act.fire {
            Some(p) => {
                x.shot = Some(p.0 as i128);
                cond_val(&c.require, &x)
            }
            None => true,
        },
        Trigger::DrivesForward | Trigger::DrivesBackward => match act.ahead {
            Some(a) if (c.when == Trigger::DrivesForward && a > 0) || (c.when == Trigger::DrivesBackward && a < 0) => {
                x.moved = Some(a as i128);
                cond_val(&c.require, &x)
            }
            _ => true,
        },
        Trigger::Always => cond_val(&c.require, &x),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use robocheck_arena::Surroundings;
    use robocode::robots::safe_shooter;
    use robocode::strategy::{Action, Observation, TargetInfo};
    use robocode::units::{Angle, Energy, GunHeat, Power};

    fn clause(json: &str) -> Result<Clause, String> {
        serde_json::from_str::<Clause>(json).map_err(|e| e.to_string())
    }

    #[test]
    fn parses_reference_s1() {
        let s1 = reference_clauses("S1");
        assert_eq!(s1.len(), 1);
        assert_eq!(s1[0].when, Trigger::Fires);
        let Cond::All(parts) = &s1[0].require else { panic!("S1 is an `all`") };
        assert_eq!(parts.len(), 3);
        assert_eq!(starter_rules().iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["S1", "S2", "S3"]);
        assert_eq!(starter_rules()[0].text, "Never fire a shot you can't afford.");
    }

    #[test]
    fn rejects_unknown_key() {
        assert!(clause(r#"{"when":"fires","require":{"pred":"aimed"},"note":"x"}"#).is_err());
        assert!(clause(r#"{"when":"fires","require":{"pred":"aimed","extra":1}}"#).is_err());
        assert!(clause(r#"{"when":"fires","require":{"cmp":{"left":{"num":1},"op":"<","right":{"num":2},"x":0}}}"#).is_err());
    }

    #[test]
    fn rejects_shot_power_outside_fires() {
        let c = clause(r#"{"when":"always","require":{"cmp":{"left":{"shot_power":true},"op":"<","right":{"num":5}}}}"#).unwrap();
        assert!(validate(&c).unwrap_err().contains("shot_power"));
        let c = clause(r#"{"when":"fires","require":{"cmp":{"left":{"move_distance":true},"op":"<","right":{"num":5}}}}"#).unwrap();
        assert!(validate(&c).unwrap_err().contains("move_distance"));
        let ok = clause(r#"{"when":"drives_forward","require":{"cmp":{"left":{"move_distance":true},"op":"<","right":{"num":5}}}}"#).unwrap();
        assert!(validate(&ok).is_ok());
    }

    fn nested_not(levels: usize) -> String {
        let mut c = r#"{"pred":"aimed"}"#.to_string();
        for _ in 1..levels {
            c = format!(r#"{{"not":{c}}}"#);
        }
        format!(r#"{{"when":"always","require":{c}}}"#)
    }

    #[test]
    fn rejects_depth_7() {
        assert!(validate(&clause(&nested_not(6)).unwrap()).is_ok());
        assert!(validate(&clause(&nested_not(7)).unwrap()).unwrap_err().contains("deep"));
    }

    #[test]
    fn rejects_41_nodes() {
        // An `all` with n preds is n + 1 nodes.
        let preds = |n: usize| vec![r#"{"pred":"gun_cool"}"#; n].join(",");
        let c40 = clause(&format!(r#"{{"when":"always","require":{{"all":[{}]}}}}"#, preds(39))).unwrap();
        assert!(validate(&c40).is_ok());
        let c41 = clause(&format!(r#"{{"when":"always","require":{{"all":[{}]}}}}"#, preds(40))).unwrap();
        assert!(validate(&c41).unwrap_err().contains("nodes"));
    }

    #[test]
    fn rejects_number_over_limit() {
        let c = |n: i64| clause(&format!(r#"{{"when":"always","require":{{"cmp":{{"left":{{"obs":"x"}},"op":"<","right":{{"num":{n}}}}}}}}}"#)).unwrap();
        assert!(validate(&c(1_000_000)).is_ok());
        assert!(validate(&c(-1_000_000)).is_ok());
        assert!(validate(&c(1_000_001)).unwrap_err().contains("1000000"));
    }

    #[test]
    fn renders_s1_exactly() {
        assert_eq!(
            render_verus(&reference_clauses("S1")[0]),
            "r.fire matches Some(p) ==> (((p.0 as int) >= 100) && ((p.0 as int) <= 3000) && ((obs.energy.0 as int) > (p.0 as int)))"
        );
        assert_eq!(
            render_verus(&reference_clauses("S3")[0]),
            "r.ahead matches Some(a) ==> (a > 0 ==> ((around.clearance_ahead as int) >= 80))"
        );
        assert_eq!(
            render_verus(&reference_clauses("S2")[0]),
            "r.fire matches Some(p) ==> ((obs.target matches Some(t) && t.age == 0) && aimed(*obs))"
        );
    }

    #[test]
    fn renders_target_cmp_with_binding() {
        let c = clause(r#"{"when":"always","require":{"cmp":{"left":{"target":"distance"},"op":"<","right":{"num":300}}}}"#).unwrap();
        assert_eq!(render_verus(&c), "(obs.target matches Some(t) && (t.distance as int) < 300)");
        assert!(!render_verus(&c).contains('\n'));
    }

    /// A small deterministic generator of well-formed observations.
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self, n: u64) -> u64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
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
            (Some(t), 0) => Angle(((t.bearing.0 + r.next(61) as i32 - 30) + 3600) % 3600),
            _ => Angle(r.next(3600) as i32),
        };
        Observation {
            turn: r.next(5000) as u32,
            energy: Energy(r.next(4000) as u32),
            gun_heat: GunHeat(if r.next(2) == 0 { 0 } else { r.next(1600) as u32 }),
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
    fn eval_matches_safe_shooter_on_reference_s1_s2() {
        let mut r = Lcg(11);
        let env = Surroundings::default();
        let (s1, s2) = (reference_clauses("S1"), reference_clauses("S2"));
        let mut fired = 0;
        for _ in 0..1000 {
            let obs = observation(&mut r);
            let act = safe_shooter::decide(&obs);
            fired += act.fire.is_some() as u32;
            for c in s1.iter().chain(&s2) {
                assert!(eval(c, &obs, &env, &act), "{c:?} broken by SafeShooter on {obs:?}");
            }
        }
        assert!(fired > 20, "SafeShooter fired only {fired} times: S1/S2 barely exercised");
        // And a shot SafeShooter would never take breaks S1.
        let obs = observation(&mut Lcg(3));
        let reckless = Action { fire: Some(Power(3000)), ..Action::default() };
        let broke = Observation { energy: Energy(2000), ..obs };
        assert!(!eval(&s1[0], &broke, &env, &reckless));
    }

    #[test]
    fn eval_s3_flags_driving_into_building() {
        let obs = observation(&mut Lcg(5));
        let s3 = reference_clauses("S3");
        let close_ahead = Surroundings { clearance_ahead: 50, clearance_behind: 200 };
        let forward = Action { ahead: Some(100), ..Action::default() };
        let backward = Action { ahead: Some(-100), ..Action::default() };
        assert!(!eval(&s3[0], &obs, &close_ahead, &forward));
        assert!(eval(&s3[0], &obs, &close_ahead, &backward));
        assert!(eval(&s3[1], &obs, &close_ahead, &backward));
        let close_behind = Surroundings { clearance_ahead: 200, clearance_behind: 10 };
        assert!(!eval(&s3[1], &obs, &close_behind, &backward));
        assert!(eval(&s3[0], &obs, &close_behind, &Action::default()));
    }
}
