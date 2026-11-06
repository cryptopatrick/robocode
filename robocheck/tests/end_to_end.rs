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
