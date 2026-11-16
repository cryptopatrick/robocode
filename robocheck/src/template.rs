//! The trusted template and how a spec and a candidate are slotted into it.

use sha2::{Digest, Sha256};

use crate::candidate::Candidate;
use crate::rulespec::{Clause, render_verus};

/// The trusted header: imports, the spec'd signature of `decide` (its
/// `ensures` list is `{{CLAUSES}}`), and the slots for the candidate's body
/// and helpers. Never produced by the candidate.
pub const STRATEGY_HEADER: &str =
    include_str!("../template/strategy_header.rs");
/// `src/lib.rs` of the candidate crate.
pub const LIB_RS: &str = include_str!("../template/lib.rs");

/// A reference property (PRD §8.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Property {
    pub id: &'static str,
    pub name: &'static str,
}

/// The reference suite: SafeShooter's properties, in report order.
pub const PROPERTIES: [Property; 7] = [
    Property { id: "SAFE-001", name: "Firing power is legal (0.1 to 3.0)" },
    Property {
        id: "SAFE-002",
        name: "Energy stays above zero after the shot",
    },
    Property { id: "SAFE-003", name: "The gun is cool when firing" },
    Property { id: "SAFE-004", name: "The target was scanned this turn" },
    Property { id: "SAFE-005", name: "The gun is aimed within 3.0 degrees" },
    Property {
        id: "SAFE-006",
        name: "Never drives toward a wall it is within 80 px of",
    },
    Property { id: "SAFE-007", name: "Fires if and only if it is ready" },
];

/// Phase 0's human-written reference clauses (SafeShooter's spec), with their property ids.
const REFERENCE_CLAUSES: [(&str, &str); 8] = [
    (
        "SAFE-001",
        "r.fire matches Some(p) ==> MIN_POWER.0 <= p.0 <= MAX_POWER.0",
    ),
    ("SAFE-002", "r.fire matches Some(p) ==> obs.energy.0 > p.0"),
    ("SAFE-003", "r.fire is Some ==> obs.gun_heat.0 == 0"),
    (
        "SAFE-004",
        "r.fire is Some ==> (obs.target matches Some(t) && t.age == 0)",
    ),
    ("SAFE-005", "r.fire is Some ==> aimed(*obs)"),
    (
        "SAFE-006",
        "r.ahead matches Some(a) ==> (a > 0 ==> !heading_into_near_wall(*obs, obs.heading.0 as int))",
    ),
    (
        "SAFE-006",
        "r.ahead matches Some(a) ==> (a < 0 ==> !heading_into_near_wall(*obs, reverse_heading(obs.heading.0 as int)))",
    ),
    ("SAFE-007", "r.fire is Some <==> ready(*obs)"),
];

/// One `ensures` clause and the rule it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct SpecClause {
    pub rule: String,
    pub verus: String,
}

/// The spec on `decide`: its clauses, and every rule (id, name) in report
/// order, including rules that have no clause.
#[derive(Debug, Clone, PartialEq)]
pub struct Spec {
    pub clauses: Vec<SpecClause>,
    pub rules: Vec<(String, String)>,
}

/// Phase 0's reference spec: SafeShooter's seven properties.
pub fn reference_spec() -> Spec {
    Spec {
        clauses: REFERENCE_CLAUSES
            .iter()
            .map(|(id, v)| SpecClause {
                rule: id.to_string(),
                verus: v.to_string(),
            })
            .collect(),
        rules: PROPERTIES
            .iter()
            .map(|p| (p.id.to_string(), p.name.to_string()))
            .collect(),
    }
}

/// A spec from formalised rules: `(id, name, clauses)`; a rule with no
/// clauses is listed but has nothing in the template.
pub fn generated_spec(rules: &[(String, String, Vec<Clause>)]) -> Spec {
    Spec {
        clauses: rules
            .iter()
            .flat_map(|(id, _, cs)| {
                cs.iter().map(move |c| SpecClause {
                    rule: id.clone(),
                    verus: render_verus(c),
                })
            })
            .collect(),
        rules: rules
            .iter()
            .map(|(id, name, _)| (id.clone(), name.clone()))
            .collect(),
    }
}

/// The `ensures` lines, one per clause, each ending `// <rule id>`. An empty
/// spec renders as a lone `true` (Verus needs at least one clause).
fn clause_block(spec: &Spec) -> String {
    if spec.clauses.is_empty() {
        return "        true,".to_string();
    }
    spec.clauses
        .iter()
        .map(|c| format!("        {}, // {}", c.verus, c.rule))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A candidate slotted into the template.
#[derive(Debug, Clone, PartialEq)]
pub struct Assembled {
    /// The complete `src/strategy.rs` of the candidate crate.
    pub strategy_rs: String,
    /// 1-based line in `strategy_rs` of each spec clause, with its rule id.
    pub clause_lines: Vec<(u32, String)>,
}

/// Inserts the spec, then the candidate's body and helpers, into the trusted header.
pub fn assemble(c: &Candidate, spec: &Spec) -> Assembled {
    let first = STRATEGY_HEADER
        .lines()
        .position(|l| l.trim() == "{{CLAUSES}}")
        .expect("the header has {{CLAUSES}}") as u32
        + 1;
    let header =
        STRATEGY_HEADER.replacen("{{CLAUSES}}", &clause_block(spec), 1);
    let strategy_rs = header.replacen("{{BODY}}", &c.body, 1).replacen(
        "{{HELPERS}}",
        &c.helpers,
        1,
    );
    // The clauses sit at fixed lines of the header, above the untrusted text.
    let clause_lines = spec
        .clauses
        .iter()
        .enumerate()
        .map(|(i, cl)| (first + i as u32, cl.rule.clone()))
        .collect();
    Assembled { strategy_rs, clause_lines }
}

/// `strategy_rs` with the clauses of `ids` replaced by `true`, line numbers kept.
pub fn neutralize(a: &Assembled, ids: &[&str]) -> String {
    let drop: Vec<u32> = a
        .clause_lines
        .iter()
        .filter(|(_, id)| ids.contains(&id.as_str()))
        .map(|(l, _)| *l)
        .collect();
    let mut out: Vec<String> = Vec::new();
    for (i, line) in a.strategy_rs.lines().enumerate() {
        if drop.contains(&(i as u32 + 1)) {
            let indent: String =
                line.chars().take_while(|c| c.is_whitespace()).collect();
            let id = line.rsplit("// ").next().unwrap_or("");
            out.push(format!("{indent}true, // {id}"));
        } else {
            out.push(line.to_string());
        }
    }
    let mut s = out.join("\n");
    if a.strategy_rs.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// Lower-case hex SHA-256.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// Hash of the trusted header with this spec's clauses in it.
pub fn spec_hash(spec: &Spec) -> String {
    sha256_hex(
        STRATEGY_HEADER
            .replacen("{{CLAUSES}}", &clause_block(spec), 1)
            .as_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::parse;

    fn fixture() -> Assembled {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/safe_shooter.rs"
        ))
        .unwrap();
        assemble(&parse(&src).unwrap(), &reference_spec())
    }

    /// The reference clauses exactly as Phase 0's template wrote them.
    const PHASE0_CLAUSES: [&str; 8] = [
        "        r.fire matches Some(p) ==> MIN_POWER.0 <= p.0 <= MAX_POWER.0, // SAFE-001",
        "        r.fire matches Some(p) ==> obs.energy.0 > p.0, // SAFE-002",
        "        r.fire is Some ==> obs.gun_heat.0 == 0, // SAFE-003",
        "        r.fire is Some ==> (obs.target matches Some(t) && t.age == 0), // SAFE-004",
        "        r.fire is Some ==> aimed(*obs), // SAFE-005",
        "        r.ahead matches Some(a) ==> (a > 0 ==> !heading_into_near_wall(*obs, obs.heading.0 as int)), // SAFE-006",
        "        r.ahead matches Some(a) ==> (a < 0 ==> !heading_into_near_wall(*obs, reverse_heading(obs.heading.0 as int))), // SAFE-006",
        "        r.fire is Some <==> ready(*obs), // SAFE-007",
    ];

    #[test]
    fn reference_spec_keeps_phase0_clauses() {
        let a = fixture();
        let lines: Vec<&str> = a
            .clause_lines
            .iter()
            .map(|(l, _)| a.strategy_rs.lines().nth(*l as usize - 1).unwrap())
            .collect();
        assert_eq!(lines, PHASE0_CLAUSES);
        assert_eq!(reference_spec().rules.len(), 7);
    }

    #[test]
    fn generated_spec_marks_each_clause() {
        use crate::rulespec::{reference_clauses, render_verus};
        let rules = vec![
            ("S1".to_string(), "afford".to_string(), reference_clauses("S1")),
            ("S3".to_string(), "walls".to_string(), reference_clauses("S3")),
            ("U1".to_string(), "not formalised".to_string(), Vec::new()),
        ];
        let spec = generated_spec(&rules);
        assert_eq!(spec.rules.len(), 3);
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/safe_shooter.rs"
        ))
        .unwrap();
        let a = assemble(&parse(&src).unwrap(), &spec);
        let ids: Vec<&str> =
            a.clause_lines.iter().map(|(_, id)| id.as_str()).collect();
        assert_eq!(ids, ["S1", "S3", "S3"]);
        let s3 = reference_clauses("S3");
        for ((line, id), verus) in a.clause_lines.iter().zip([
            render_verus(&reference_clauses("S1")[0]),
            render_verus(&s3[0]),
            render_verus(&s3[1]),
        ]) {
            let text = a.strategy_rs.lines().nth(*line as usize - 1).unwrap();
            assert_eq!(text, format!("        {verus}, // {id}"));
        }
        assert!(!a.strategy_rs.contains("// U1"));
        assert_ne!(spec_hash(&spec), spec_hash(&reference_spec()));
    }

    #[test]
    fn empty_spec_still_assembles() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/safe_shooter.rs"
        ))
        .unwrap();
        let a = assemble(&parse(&src).unwrap(), &generated_spec(&[]));
        assert!(a.clause_lines.is_empty());
        assert!(!a.strategy_rs.contains("{{CLAUSES}}"));
    }

    #[test]
    fn every_property_has_a_clause() {
        let a = fixture();
        assert_eq!(a.clause_lines.len(), 8);
        for p in PROPERTIES {
            let n = a.clause_lines.iter().filter(|(_, id)| id == p.id).count();
            assert_eq!(n, if p.id == "SAFE-006" { 2 } else { 1 }, "{}", p.id);
        }
    }

    #[test]
    fn clause_lines_point_at_their_marker() {
        let a = fixture();
        for (line, id) in &a.clause_lines {
            let text = a.strategy_rs.lines().nth(*line as usize - 1).unwrap();
            assert!(
                text.ends_with(&format!("// {id}")),
                "line {line}: {text}"
            );
        }
    }

    #[test]
    fn body_and_helpers_are_inserted_once() {
        let a = fixture();
        assert!(
            !a.strategy_rs.contains("{{BODY}}")
                && !a.strategy_rs.contains("{{HELPERS}}")
        );
        assert_eq!(
            a.strategy_rs
                .matches("let (ahead, turn_body) = movement(obs);")
                .count(),
            1
        );
        assert_eq!(a.strategy_rs.matches("fn strafe_turn(").count(), 1);
    }

    #[test]
    fn neutralize_keeps_lines_and_drops_clauses() {
        let a = fixture();
        let n = neutralize(&a, &["SAFE-002"]);
        assert_eq!(n.lines().count(), a.strategy_rs.lines().count());
        for (line, id) in &a.clause_lines {
            let text = n.lines().nth(*line as usize - 1).unwrap();
            if id == "SAFE-002" {
                assert_eq!(text.trim(), "true, // SAFE-002");
            } else {
                assert_eq!(
                    text,
                    a.strategy_rs.lines().nth(*line as usize - 1).unwrap()
                );
            }
        }
    }

    #[test]
    fn hashes_are_stable() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(fixture().strategy_rs, fixture().strategy_rs);
        assert_eq!(spec_hash(&reference_spec()).len(), 64);
    }
}
