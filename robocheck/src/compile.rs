//! Plain (non-Verus) build of the candidate crate.

use std::path::Path;
use std::process::Command;

/// Result of the plain build.
#[derive(Debug, Clone, PartialEq)]
pub enum BuildOutcome {
    Ok,
    /// The compiler's output.
    Failed { log: String },
}

/// `cargo build --release --bins` (the one battle binary) in the candidate crate, without the
/// `verus` feature (specs vanish; the code is what runs in battles).
pub fn build(dir: &Path, target_dir: &Path) -> BuildOutcome {
    let out = Command::new("cargo")
        .args(["build", "--release", "--locked", "--bins"])
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", target_dir)
        .output();
    match out {
        Ok(o) if o.status.success() => BuildOutcome::Ok,
        Ok(o) => BuildOutcome::Failed {
            log: format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)),
        },
        Err(e) => BuildOutcome::Failed { log: format!("could not run cargo: {e}") },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::parse;
    use crate::template::assemble;
    use crate::workspace::{materialize, plain_target_dir, repo_root, tests::{shared_target, temp_dir}};

    /// `tag` keeps each test's directory its own (tests run in parallel).
    fn materialized(fixture: &str, tag: &str) -> std::path::PathBuf {
        let src = std::fs::read_to_string(repo_root().join(format!("robocheck/fixtures/{fixture}.rs"))).unwrap();
        let dir = temp_dir(&format!("{fixture}-{tag}"));
        materialize(&dir, &assemble(&parse(&src).unwrap()).strategy_rs, &repo_root()).unwrap();
        dir
    }

    #[test]
    fn safe_shooter_fixture_builds() {
        let _guard = shared_target();
        let dir = materialized("safe_shooter", "builds");
        assert_eq!(build(&dir, &plain_target_dir()), BuildOutcome::Ok);
    }

    /// A candidate whose files are older than an earlier candidate's build
    /// must still be compiled, never served the earlier candidate's artifacts.
    #[test]
    fn stale_artifacts_are_not_reused() {
        let target = std::env::temp_dir().join(format!("robocheck-test-stale-target-{}", std::process::id()));
        let good = materialized("safe_shooter", "stale");
        assert_eq!(build(&good, &target), BuildOutcome::Ok);
        let bad = materialized("type_error", "stale");
        let status = std::process::Command::new("touch")
            .args(["-t", "202001010000"])
            .args(["Cargo.toml", "Cargo.lock", "src/lib.rs", "src/strategy.rs", "src/bin/battle.rs"].map(|f| bad.join(f)))
            .status()
            .unwrap();
        assert!(status.success());
        assert!(matches!(build(&bad, &target), BuildOutcome::Failed { .. }), "type_error reused safe_shooter's build");
    }

    #[test]
    fn type_error_is_build_failed() {
        let _guard = shared_target();
        let dir = materialized("type_error", "fails");
        match build(&dir, &plain_target_dir()) {
            BuildOutcome::Failed { log } => assert!(log.contains("mismatched types"), "{log}"),
            BuildOutcome::Ok => panic!("type_error must not build"),
        }
    }
}
