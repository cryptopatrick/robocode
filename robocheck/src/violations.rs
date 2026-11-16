//! Finds the turns where a candidate broke a rule, from the battle's decision
//! log (`decisions-<arena>.jsonl`, one robocheck-arena `DecisionRecord` per line).

use robocheck_arena::Surroundings;
use robocode::strategy::{Action, Observation, TargetInfo};
use robocode::units::{Angle, Energy, GunHeat, Power};
use serde::{Deserialize, Serialize};

use crate::rulespec::{Clause, eval};

/// A rule the candidate broke in one battle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Violation {
    pub rule: String,
    /// The first turn that broke it.
    pub first_turn: u32,
    /// How many turns broke it.
    pub turns: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rec {
    turn: u32,
    obs: Obs,
    env: Env,
    action: Act,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Obs {
    turn: u32,
    energy: u32,
    gun_heat: u32,
    heading: i32,
    gun_heading: i32,
    radar_heading: i32,
    velocity: i32,
    x: i32,
    y: i32,
    field_width: i32,
    field_height: i32,
    target: Option<Tgt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tgt {
    bearing: i32,
    distance: i32,
    energy: u32,
    heading: i32,
    velocity: i32,
    age: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Env {
    clearance_ahead: i32,
    clearance_behind: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Act {
    fire: Option<u32>,
    ahead: Option<i32>,
    turn_body: Option<i32>,
    turn_gun: Option<i32>,
    turn_radar: Option<i32>,
}

impl Rec {
    fn parts(&self) -> (Observation, Surroundings, Action) {
        let o = &self.obs;
        let obs = Observation {
            turn: o.turn,
            energy: Energy(o.energy),
            gun_heat: GunHeat(o.gun_heat),
            heading: Angle(o.heading),
            gun_heading: Angle(o.gun_heading),
            radar_heading: Angle(o.radar_heading),
            velocity: o.velocity,
            x: o.x,
            y: o.y,
            field_width: o.field_width,
            field_height: o.field_height,
            target: o.target.as_ref().map(|t| TargetInfo {
                bearing: Angle(t.bearing),
                distance: t.distance,
                energy: Energy(t.energy),
                heading: Angle(t.heading),
                velocity: t.velocity,
                age: t.age,
            }),
        };
        let env = Surroundings {
            clearance_ahead: self.env.clearance_ahead,
            clearance_behind: self.env.clearance_behind,
        };
        let a = &self.action;
        let act = Action {
            fire: a.fire.map(Power),
            ahead: a.ahead,
            turn_body: a.turn_body,
            turn_gun: a.turn_gun,
            turn_radar: a.turn_radar,
        };
        (obs, env, act)
    }
}

/// For each rule (id, its clauses) the candidate broke at least once: the
/// first turn and how many turns. A turn breaks a rule if it breaks any of
/// its clauses. Rules come back in the order given; unbroken rules are left out.
pub fn find_violations(
    decisions_jsonl: &str,
    rules: &[(String, Vec<Clause>)],
) -> Result<Vec<Violation>, String> {
    let mut found: Vec<Option<Violation>> = vec![None; rules.len()];
    for (n, line) in decisions_jsonl
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        let rec: Rec = serde_json::from_str(line)
            .map_err(|e| format!("decision log line {}: {e}", n + 1))?;
        let (obs, env, act) = rec.parts();
        for (slot, (id, clauses)) in found.iter_mut().zip(rules) {
            if clauses.iter().any(|c| !eval(c, &obs, &env, &act)) {
                match slot {
                    Some(v) => v.turns += 1,
                    None => {
                        *slot = Some(Violation {
                            rule: id.clone(),
                            first_turn: rec.turn,
                            turns: 1,
                        })
                    }
                }
            }
        }
    }
    Ok(found.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rulespec::reference_clauses;

    fn line(
        turn: u32,
        energy: u32,
        fire: Option<u32>,
        ahead: Option<i32>,
        clearance_ahead: i32,
    ) -> String {
        let opt = |v: Option<String>| v.unwrap_or_else(|| "null".into());
        format!(
            r#"{{"turn":{turn},"obs":{{"turn":{turn},"energy":{energy},"gun_heat":0,"heading":900,"gun_heading":900,"radar_heading":0,"velocity":0,"x":400,"y":300,"field_width":800,"field_height":600,"target":{{"bearing":900,"distance":200,"energy":50000,"heading":0,"velocity":0,"age":0}}}},"env":{{"clearance_ahead":{clearance_ahead},"clearance_behind":300}},"action":{{"fire":{},"ahead":{},"turn_body":null,"turn_gun":null,"turn_radar":null}}}}"#,
            opt(fire.map(|p| p.to_string())),
            opt(ahead.map(|a| a.to_string()))
        )
    }

    fn rules() -> Vec<(String, Vec<crate::rulespec::Clause>)> {
        ["S1", "S2", "S3"]
            .iter()
            .map(|id| (id.to_string(), reference_clauses(id)))
            .collect()
    }

    #[test]
    fn finds_first_turn_and_count_per_rule() {
        let log = [
            line(1, 50_000, None, Some(100), 500),
            line(2, 2_000, Some(3000), Some(100), 60), // S1 (can't afford) and S3 (too close)
            line(3, 50_000, Some(1000), Some(100), 70), // S3
            line(4, 50_000, Some(1000), Some(-100), 10), // backward: S3 keeps
        ]
        .join("\n");
        let v = find_violations(&log, &rules()).unwrap();
        assert_eq!(
            v,
            vec![
                Violation { rule: "S1".into(), first_turn: 2, turns: 1 },
                Violation { rule: "S3".into(), first_turn: 2, turns: 2 },
            ]
        );
    }

    #[test]
    fn clean_log_has_no_violations() {
        let log = [
            line(1, 50_000, Some(1000), Some(100), 500),
            line(2, 50_000, None, None, 0),
        ]
        .join("\n");
        assert!(find_violations(&log, &rules()).unwrap().is_empty());
    }

    #[test]
    fn malformed_log_is_an_error() {
        assert!(find_violations("{not json", &rules()).is_err());
        assert!(find_violations("", &rules()).unwrap().is_empty());
    }
}
