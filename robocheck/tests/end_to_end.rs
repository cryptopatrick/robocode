//! End-to-end checks against the real Verus. Run with
//! `cargo test -p robocheck --test end_to_end -- --ignored` (needs Verus; see docs/verus-integration.md).

use std::time::Duration;

use robocheck::candidate::parse;
use robocheck::template::assemble;
use robocheck::verify::{CargoVerus, PropertyStatus, verify_properties};
use robocheck::workspace::{materialize, repo_root, verus_target_dir};

/// Verifies a fixture with real Verus; returns (property id, status) pairs.
fn verify_fixture(name: &str) -> Vec<(&'static str, PropertyStatus)> {
    let src = std::fs::read_to_string(repo_root().join(format!("robocheck/fixtures/{name}.rs"))).unwrap();
    let a = assemble(&parse(&src).unwrap());
    let dir = std::env::temp_dir().join(format!("robocheck-e2e-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    materialize(&dir, &a.strategy_rs, &repo_root()).unwrap();
    let mut v = CargoVerus {
        dir,
        target_dir: verus_target_dir(),
        verus_dir: CargoVerus::default_verus_dir(),
        timeout: Duration::from_secs(300),
    };
    let (r, logs) = verify_properties(&a, &mut v, 8);
    assert!(!logs.is_empty());
    r.iter().map(|p| (p.id, p.status)).collect()
}

fn expect(name: &str, rejected: &[&str]) {
    for (id, s) in verify_fixture(name) {
        let want = if rejected.contains(&id) { PropertyStatus::Rejected } else { PropertyStatus::Proved };
        assert_eq!(s, want, "{name}: {id}");
    }
}

#[test]
#[ignore = "needs Verus"]
fn safe_shooter_proves_everything() {
    expect("safe_shooter", &[]);
}

#[test]
#[ignore = "needs Verus"]
fn energy_positive_is_rejected_on_energy_and_readiness() {
    expect("energy_positive", &["SAFE-002", "SAFE-007"]);
}

#[test]
#[ignore = "needs Verus"]
fn never_fires_is_rejected_on_readiness_only() {
    expect("never_fires", &["SAFE-007"]);
}

#[test]
#[ignore = "needs Verus"]
fn wall_blind_is_rejected_on_walls_only() {
    expect("wall_blind", &["SAFE-006"]);
}

/// The whole pipeline through the CLI, with real Verus.
fn run_cli(name: &str) -> serde_json::Value {
    let out = std::env::temp_dir().join(format!("robocheck-e2e-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let fixture = repo_root().join(format!("robocheck/fixtures/{name}.rs"));
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_robocheck"))
        .args(["run", fixture.to_str().unwrap(), "--turns", "1500", "--out"])
        .arg(&out)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(0));
    serde_json::from_str(&std::fs::read_to_string(out.join("evidence.json")).unwrap()).unwrap()
}

#[test]
#[ignore = "needs Verus"]
fn cli_safe_shooter_proves_everything_and_battles_the_same_artifact() {
    let ev = run_cli("safe_shooter");
    for p in ev["verification"]["properties"].as_array().unwrap() {
        assert_eq!(p["status"], "proved", "{p}");
    }
    assert_eq!(ev["battle"]["status"], "ok");
    assert_eq!(ev["battle"]["same_artifact"], true);
    assert_eq!(ev["battle"]["artifact_hash"], ev["artifact_hash"]);
    assert!(ev["toolchain"]["verus"].as_str().unwrap().contains("0.2026"));
}

#[test]
#[ignore = "needs Verus"]
fn cli_rejected_candidate_still_battles() {
    let ev = run_cli("energy_positive");
    let rejected: Vec<&str> = ev["verification"]["properties"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["status"] == "rejected")
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert_eq!(rejected, vec!["SAFE-002", "SAFE-007"]);
    assert_eq!(ev["battle"]["status"], "ok");
    assert_eq!(ev["battle"]["same_artifact"], true);
}

/// Review finding 3: a candidate that hangs is recorded as a timed-out
/// battle (and unknown verification), and robocheck finishes.
#[test]
#[ignore = "needs Verus"]
fn cli_hanging_candidate_times_out() {
    let out = std::env::temp_dir().join(format!("robocheck-e2e-cli-hangs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let fixture = repo_root().join("robocheck/fixtures/hangs.rs");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_robocheck"))
        .args(["run", fixture.to_str().unwrap(), "--battle-timeout", "3", "--out"])
        .arg(&out)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(0));
    let ev: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(out.join("evidence.json")).unwrap()).unwrap();
    assert_eq!(ev["battle"]["status"], "timed_out");
    for p in ev["verification"]["properties"].as_array().unwrap() {
        assert_ne!(p["status"], "proved", "{p}");
    }
}
