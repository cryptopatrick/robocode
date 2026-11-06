//! Verification stage: Verus checks the candidate crate against the
//! reference spec, one result per property.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::template::{Assembled, PROPERTIES, neutralize};

/// Result states (PRD §7.5).
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum PropertyStatus {
    Proved,
    Rejected,
    Unknown,
    NotChecked,
    BuildFailed,
    TimedOut,
}

/// One Verus run over the candidate crate.
#[derive(Debug, Clone, PartialEq)]
pub struct VerusRun {
    /// Verus finished with zero errors.
    pub success: bool,
    pub timed_out: bool,
    pub log: String,
}

/// Runs Verus on a given `strategy.rs`. `Err` means Verus could not be run at all.
pub trait Verifier {
    fn run(&mut self, strategy_rs: &str) -> Result<VerusRun, String>;
}

/// An error in Verus's output.
#[derive(Debug, Clone, PartialEq)]
pub enum Failure {
    /// `postcondition not satisfied` at this line of `src/strategy.rs`.
    Clause { line: u32 },
    /// Any other error, with its first line.
    Other { summary: String },
}

/// Reads Verus's text diagnostics (run with `--multiple-errors 20`).
pub fn parse_failures(log: &str) -> Vec<Failure> {
    let lines: Vec<&str> = log.lines().collect();
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let Some(msg) = l.strip_prefix("error: ") else { continue };
        if msg.starts_with("could not compile") || msg.starts_with("aborting due to") {
            continue;
        }
        let location = lines[i + 1..].iter().take(3).find_map(|n| n.trim_start().strip_prefix("--> "));
        let line = location
            .and_then(|loc| loc.split_once("src/strategy.rs:"))
            .and_then(|(_, rest)| rest.split(':').next()?.parse::<u32>().ok());
        match (msg == "postcondition not satisfied", line) {
            (true, Some(line)) => out.push(Failure::Clause { line }),
            _ => out.push(Failure::Other { summary: msg.to_string() }),
        }
    }
    out
}

/// The verdict on one reference property.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PropertyResult {
    pub id: &'static str,
    pub name: &'static str,
    pub status: PropertyStatus,
    pub note: String,
}

/// Verifies the reference properties one by one: a property is `proved` only
/// from a run with zero errors. Each round, clauses Verus rejected are
/// replaced by `true` and the rest re-verified. Returns the results in
/// [`PROPERTIES`] order and the log of every round.
pub fn verify_properties(a: &Assembled, v: &mut dyn Verifier, max_rounds: usize) -> (Vec<PropertyResult>, Vec<String>) {
    let mut rejected: Vec<(&'static str, u32)> = Vec::new();
    let mut logs = Vec::new();
    let mut rest: Option<(PropertyStatus, String)> = None;
    for _ in 0..max_rounds {
        let ids: Vec<&str> = rejected.iter().map(|(id, _)| *id).collect();
        let run = match v.run(&neutralize(a, &ids)) {
            Ok(run) => run,
            Err(e) => {
                rest = Some((PropertyStatus::NotChecked, e));
                break;
            }
        };
        logs.push(run.log.clone());
        if run.timed_out {
            rest = Some((PropertyStatus::TimedOut, "Verus exceeded its time limit".into()));
            break;
        }
        if run.success {
            rest = Some((PropertyStatus::Proved, String::new()));
            break;
        }
        let failures = parse_failures(&run.log);
        let mut others = Vec::new();
        let mut new = Vec::new();
        for f in &failures {
            match f {
                Failure::Clause { line } => match a.clause_lines.iter().find(|(l, _)| l == line) {
                    Some((_, id)) if !ids.contains(id) => new.push((*id, *line)),
                    Some(_) => {}
                    None => others.push(format!("postcondition not satisfied on line {line}, not a reference clause")),
                },
                Failure::Other { summary } => others.push(summary.clone()),
            }
        }
        if failures.is_empty() {
            others.push("Verus failed without a recognised verification error".into());
        }
        if !others.is_empty() {
            rest = Some((PropertyStatus::Unknown, others.join("; ")));
            break;
        }
        if new.is_empty() {
            rest = Some((PropertyStatus::Unknown, "Verus kept rejecting clauses that were already set aside".into()));
            break;
        }
        rejected.extend(new);
    }
    let (rest_status, rest_note) =
        rest.unwrap_or((PropertyStatus::Unknown, format!("not settled within {max_rounds} Verus runs")));
    let results = PROPERTIES
        .iter()
        .map(|p| {
            let lines: Vec<String> =
                rejected.iter().filter(|(id, _)| *id == p.id).map(|(_, l)| l.to_string()).collect();
            if lines.is_empty() {
                PropertyResult { id: p.id, name: p.name, status: rest_status, note: rest_note.clone() }
            } else {
                PropertyResult {
                    id: p.id,
                    name: p.name,
                    status: PropertyStatus::Rejected,
                    note: format!("Verus: postcondition not satisfied (strategy.rs line {})", lines.join(", ")),
                }
            }
        })
        .collect();
    (results, logs)
}

/// Runs `cargo verus verify --features verus` in a materialized candidate crate.
#[derive(Debug, Clone)]
pub struct CargoVerus {
    pub dir: PathBuf,
    pub target_dir: PathBuf,
    /// Folder holding `cargo-verus`, `verus` and `z3`.
    pub verus_dir: PathBuf,
    pub timeout: Duration,
}

impl CargoVerus {
    /// `ROBOCHECK_VERUS_DIR`, or `~/.verus/verus-arm64-macos`.
    pub fn default_verus_dir() -> PathBuf {
        std::env::var_os("ROBOCHECK_VERUS_DIR").map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".verus/verus-arm64-macos")
        })
    }
}

impl Verifier for CargoVerus {
    fn run(&mut self, strategy_rs: &str) -> Result<VerusRun, String> {
        if !self.verus_dir.join("cargo-verus").exists() {
            return Err(format!("cargo-verus not found in {} (set ROBOCHECK_VERUS_DIR)", self.verus_dir.display()));
        }
        std::fs::write(self.dir.join("src/strategy.rs"), strategy_rs).map_err(|e| e.to_string())?;
        let log_path = self.dir.join("verus-run.log");
        let log_file = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;
        let path = format!("{}:{}", self.verus_dir.display(), std::env::var("PATH").unwrap_or_default());
        let mut child = Command::new("cargo")
            .args(["verus", "verify", "--features", "verus", "--", "--multiple-errors", "20"])
            .current_dir(&self.dir)
            .env("PATH", path)
            .env("CARGO_TARGET_DIR", &self.target_dir)
            .stdout(Stdio::from(log_file.try_clone().map_err(|e| e.to_string())?))
            .stderr(Stdio::from(log_file))
            .spawn()
            .map_err(|e| format!("could not run cargo verus: {e}"))?;
        let start = Instant::now();
        let status = loop {
            if let Some(s) = child.try_wait().map_err(|e| e.to_string())? {
                break Some(s);
            }
            if start.elapsed() > self.timeout {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            std::thread::sleep(Duration::from_millis(200));
        };
        let log = std::fs::read_to_string(&log_path).unwrap_or_default();
        Ok(match status {
            Some(s) => VerusRun { success: s.success(), timed_out: false, log },
            None => VerusRun { success: false, timed_out: true, log },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::parse;
    use crate::template::assemble;
    use std::collections::VecDeque;

    fn log(name: &str) -> String {
        std::fs::read_to_string(format!("{}/tests/data/{name}.log", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    fn assembled() -> Assembled {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/safe_shooter.rs")).unwrap();
        assemble(&parse(&src).unwrap())
    }

    fn line_of(a: &Assembled, id: &str) -> u32 {
        a.clause_lines.iter().find(|(_, i)| *i == id).unwrap().0
    }

    fn clause_failure_log(lines: &[u32]) -> String {
        lines
            .iter()
            .map(|l| format!("error: postcondition not satisfied\n  --> src/strategy.rs:{l}:9\n   |\n"))
            .collect::<String>()
            + "verification results:: 5 verified, 1 errors\n"
    }

    /// A scripted verifier: returns the queued runs in order and records its inputs.
    struct Fake {
        runs: VecDeque<Result<VerusRun, String>>,
        inputs: Vec<String>,
    }

    impl Fake {
        fn new(runs: Vec<Result<VerusRun, String>>) -> Self {
            Fake { runs: runs.into(), inputs: Vec::new() }
        }
    }

    impl Verifier for Fake {
        fn run(&mut self, strategy_rs: &str) -> Result<VerusRun, String> {
            self.inputs.push(strategy_rs.to_string());
            self.runs.pop_front().expect("no more scripted runs")
        }
    }

    fn ok() -> Result<VerusRun, String> {
        Ok(VerusRun { success: true, timed_out: false, log: "verification results:: 6 verified, 0 errors".into() })
    }

    fn failed(log: String) -> Result<VerusRun, String> {
        Ok(VerusRun { success: false, timed_out: false, log })
    }

    fn statuses(r: &[PropertyResult]) -> Vec<(&'static str, PropertyStatus)> {
        r.iter().map(|p| (p.id, p.status)).collect()
    }

    #[test]
    fn parser_finds_clause_lines() {
        let a = assembled();
        let f = parse_failures(&log("energy_positive"));
        assert_eq!(f, vec![Failure::Clause { line: line_of(&a, "SAFE-002") }, Failure::Clause { line: line_of(&a, "SAFE-007") }]);
    }

    #[test]
    fn parser_reports_other_errors() {
        let f = parse_failures(&log("overflow"));
        assert!(f.iter().any(|x| matches!(x, Failure::Other { summary } if summary.contains("overflow"))), "{f:?}");
        assert!(!f.iter().any(|x| matches!(x, Failure::Clause { .. })));
    }

    #[test]
    fn parser_ignores_a_clean_log() {
        assert!(parse_failures(&log("safe_shooter")).is_empty());
    }

    #[test]
    fn all_proved_on_clean_run() {
        let mut v = Fake::new(vec![ok()]);
        let (r, logs) = verify_properties(&assembled(), &mut v, 8);
        assert!(r.iter().all(|p| p.status == PropertyStatus::Proved));
        assert_eq!((r.len(), logs.len(), v.inputs.len()), (7, 1, 1));
    }

    #[test]
    fn rejected_clause_then_rest_proved() {
        let a = assembled();
        let mut v = Fake::new(vec![failed(clause_failure_log(&[line_of(&a, "SAFE-002")])), ok()]);
        let (r, _) = verify_properties(&a, &mut v, 8);
        for (id, s) in statuses(&r) {
            assert_eq!(s, if id == "SAFE-002" { PropertyStatus::Rejected } else { PropertyStatus::Proved }, "{id}");
        }
        assert!(v.inputs[1].contains("true, // SAFE-002"));
        assert!(!v.inputs[0].contains("true, // SAFE-002"));
    }

    #[test]
    fn unattributed_errors_make_unknown() {
        let mut v = Fake::new(vec![failed(log("overflow"))]);
        let (r, _) = verify_properties(&assembled(), &mut v, 8);
        assert!(r.iter().all(|p| p.status == PropertyStatus::Unknown), "{:?}", statuses(&r));
        assert!(r[0].note.contains("overflow"));
    }

    #[test]
    fn failure_outside_the_reference_clauses_is_unknown() {
        // e.g. a helper's own ensures: a postcondition on a line that is not a reference clause
        let mut v = Fake::new(vec![failed(clause_failure_log(&[120]))]);
        let (r, _) = verify_properties(&assembled(), &mut v, 8);
        assert!(r.iter().all(|p| p.status == PropertyStatus::Unknown), "{:?}", statuses(&r));
    }

    #[test]
    fn missing_verus_is_not_checked() {
        let mut v = Fake::new(vec![Err("cargo verus not found".into())]);
        let (r, _) = verify_properties(&assembled(), &mut v, 8);
        assert!(r.iter().all(|p| p.status == PropertyStatus::NotChecked && p.note.contains("cargo verus not found")));
    }

    #[test]
    fn timeout_is_not_a_verdict() {
        let a = assembled();
        let timeout = || Ok(VerusRun { success: false, timed_out: true, log: String::new() });
        let (r, _) = verify_properties(&a, &mut Fake::new(vec![timeout()]), 8);
        assert!(r.iter().all(|p| p.status == PropertyStatus::TimedOut));
        let mut v = Fake::new(vec![failed(clause_failure_log(&[line_of(&a, "SAFE-007")])), timeout()]);
        let (r, _) = verify_properties(&a, &mut v, 8);
        for (id, s) in statuses(&r) {
            assert_eq!(s, if id == "SAFE-007" { PropertyStatus::Rejected } else { PropertyStatus::TimedOut }, "{id}");
        }
    }

    #[test]
    fn rounds_are_bounded() {
        let a = assembled();
        let fail = || failed(clause_failure_log(&[line_of(&a, "SAFE-001")]));
        let (r, logs) = verify_properties(&a, &mut Fake::new(vec![fail(), fail()]), 2);
        assert_eq!(logs.len(), 2);
        assert!(r.iter().any(|p| p.status == PropertyStatus::Unknown));
    }
}
