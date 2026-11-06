//! The trusted template and how a candidate is slotted into it.

use sha2::{Digest, Sha256};

use crate::candidate::Candidate;

/// The trusted header: imports, the reference spec on `decide`, and the
/// `Candidate` robot wrapper. Never produced by the candidate.
pub const STRATEGY_HEADER: &str = include_str!("../template/strategy_header.rs");
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
    Property { id: "SAFE-002", name: "Energy stays above zero after the shot" },
    Property { id: "SAFE-003", name: "The gun is cool when firing" },
    Property { id: "SAFE-004", name: "The target was scanned this turn" },
    Property { id: "SAFE-005", name: "The gun is aimed within 3.0 degrees" },
    Property { id: "SAFE-006", name: "Never drives toward a wall it is within 80 px of" },
    Property { id: "SAFE-007", name: "Fires if and only if it is ready" },
];

/// A candidate slotted into the template.
#[derive(Debug, Clone, PartialEq)]
pub struct Assembled {
    /// The complete `src/strategy.rs` of the candidate crate.
    pub strategy_rs: String,
    /// 1-based line in `strategy_rs` of each reference clause, with its property id.
    pub clause_lines: Vec<(u32, &'static str)>,
}

/// Inserts the candidate's body and helpers into the trusted header.
pub fn assemble(c: &Candidate) -> Assembled {
    let strategy_rs = STRATEGY_HEADER.replacen("{{BODY}}", &c.body, 1).replacen("{{HELPERS}}", &c.helpers, 1);
    // Clause markers live in the header, above the body, so the first
    // occurrence of each marker is the clause (later text is untrusted).
    let header_lines = STRATEGY_HEADER.lines().position(|l| l.contains("{{BODY}}")).unwrap_or(0);
    let clause_lines = strategy_rs
        .lines()
        .take(header_lines)
        .enumerate()
        .filter_map(|(i, l)| {
            PROPERTIES.iter().find(|p| l.ends_with(&format!("// {}", p.id))).map(|p| (i as u32 + 1, p.id))
        })
        .collect();
    Assembled { strategy_rs, clause_lines }
}

/// `strategy_rs` with the clauses of `ids` replaced by `true`, line numbers kept.
pub fn neutralize(a: &Assembled, ids: &[&str]) -> String {
    let drop: Vec<u32> = a.clause_lines.iter().filter(|(_, id)| ids.contains(id)).map(|(l, _)| *l).collect();
    let mut out: Vec<String> = Vec::new();
    for (i, line) in a.strategy_rs.lines().enumerate() {
        if drop.contains(&(i as u32 + 1)) {
            let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
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

/// Hash of the trusted header, which contains the reference spec.
pub fn spec_hash() -> String {
    sha256_hex(STRATEGY_HEADER.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::parse;

    fn fixture() -> Assembled {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/safe_shooter.rs")).unwrap();
        assemble(&parse(&src).unwrap())
    }

    #[test]
    fn every_property_has_a_clause() {
        let a = fixture();
        assert_eq!(a.clause_lines.len(), 8);
        for p in PROPERTIES {
            let n = a.clause_lines.iter().filter(|(_, id)| *id == p.id).count();
            assert_eq!(n, if p.id == "SAFE-006" { 2 } else { 1 }, "{}", p.id);
        }
    }

    #[test]
    fn clause_lines_point_at_their_marker() {
        let a = fixture();
        for (line, id) in &a.clause_lines {
            let text = a.strategy_rs.lines().nth(*line as usize - 1).unwrap();
            assert!(text.ends_with(&format!("// {id}")), "line {line}: {text}");
        }
    }

    #[test]
    fn body_and_helpers_are_inserted_once() {
        let a = fixture();
        assert!(!a.strategy_rs.contains("{{BODY}}") && !a.strategy_rs.contains("{{HELPERS}}"));
        assert_eq!(a.strategy_rs.matches("let (ahead, turn_body) = movement(obs);").count(), 1);
        assert_eq!(a.strategy_rs.matches("fn strafe_turn(").count(), 1);
    }

    #[test]
    fn neutralize_keeps_lines_and_drops_clauses() {
        let a = fixture();
        let n = neutralize(&a, &["SAFE-002"]);
        assert_eq!(n.lines().count(), a.strategy_rs.lines().count());
        for (line, id) in &a.clause_lines {
            let text = n.lines().nth(*line as usize - 1).unwrap();
            if *id == "SAFE-002" {
                assert_eq!(text.trim(), "true, // SAFE-002");
            } else {
                assert_eq!(text, a.strategy_rs.lines().nth(*line as usize - 1).unwrap());
            }
        }
    }

    #[test]
    fn hashes_are_stable() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(fixture().strategy_rs, fixture().strategy_rs);
        assert_eq!(spec_hash().len(), 64);
    }
}
