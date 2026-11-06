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

/// `cargo build --release --bin battle` in the candidate crate, without the
/// `verus` feature (specs vanish; the code is what runs in battles).
pub fn build(dir: &Path, target_dir: &Path) -> BuildOutcome {
    let out = Command::new("cargo")
        .args(["build", "--release", "--locked", "--bin", "battle"])
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
    use crate::workspace::{materialize, plain_target_dir, repo_root, tests::temp_dir};

    fn materialized(fixture: &str) -> std::path::PathBuf {
        let src = std::fs::read_to_string(repo_root().join(format!("robocheck/fixtures/{fixture}.rs"))).unwrap();
        let dir = temp_dir(fixture);
        materialize(&dir, &assemble(&parse(&src).unwrap()).strategy_rs, &repo_root()).unwrap();
        dir
    }

    #[test]
    fn safe_shooter_fixture_builds() {
        let dir = materialized("safe_shooter");
        assert_eq!(build(&dir, &plain_target_dir()), BuildOutcome::Ok);
    }

    #[test]
    fn type_error_is_build_failed() {
        let dir = materialized("type_error");
        match build(&dir, &plain_target_dir()) {
            BuildOutcome::Failed { log } => assert!(log.contains("mismatched types"), "{log}"),
            BuildOutcome::Ok => panic!("type_error must not build"),
        }
    }
}
