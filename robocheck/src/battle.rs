//! Battle stage: the candidate crate's battle binary fights a built-in robot.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::compile::{BuildOutcome, build};
use crate::template::sha256_hex;
use crate::workspace::crate_and_bin;

/// What the battle binary reported, plus the recording's hash.
#[derive(Debug, Clone, PartialEq)]
pub struct BattleResult {
    /// SHA-256 of the `strategy.rs` compiled into the battle binary.
    pub artifact_hash: String,
    pub winner: Option<String>,
    pub turns: u64,
    pub recording_hash: String,
}

#[derive(Deserialize)]
struct Line {
    artifact_hash: String,
    winner: Option<String>,
    turns: u64,
}

/// Builds the candidate crate in `dir` and runs its battle binary against
/// `opponent`, writing the recording to `recording_out`.
pub fn run_battle(
    dir: &Path,
    target_dir: &Path,
    opponent: &str,
    max_turns: u64,
    recording_out: &Path,
) -> Result<BattleResult, String> {
    let strategy = std::fs::read_to_string(dir.join("src/strategy.rs")).map_err(|e| e.to_string())?;
    // Named after this exact strategy.rs: if the file was changed after
    // materialize, the binary is not found rather than silently rebuilt.
    let (_, bin) = crate_and_bin(&strategy);
    if let BuildOutcome::Failed { log } = build(dir, target_dir) {
        return Err(format!("battle build failed:\n{log}"));
    }
    let exe = target_dir.join("release").join(&bin);
    let out = Command::new(&exe)
        .args(["--opponent", opponent, "--turns", &max_turns.to_string(), "--out"])
        .arg(recording_out)
        .output()
        .map_err(|e| format!("cannot run {}: {e}", exe.display()))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let line: Line = serde_json::from_slice(&out.stdout).map_err(|e| format!("unexpected battle output: {e}"))?;
    let rec = std::fs::read(recording_out).map_err(|e| e.to_string())?;
    Ok(BattleResult {
        artifact_hash: line.artifact_hash,
        winner: line.winner,
        turns: line.turns,
        recording_hash: sha256_hex(&rec),
    })
}

/// The battle must have run the implementation that was verified.
pub fn check_same_artifact(verified: &str, battled: &str) -> Result<(), String> {
    if verified == battled {
        Ok(())
    } else {
        Err(format!("verified implementation {verified} differs from the battled one {battled}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::parse;
    use crate::template::{assemble, sha256_hex};
    use crate::workspace::{materialize, plain_target_dir, repo_root, tests::temp_dir};

    fn ready(name: &str) -> (std::path::PathBuf, String) {
        let src = std::fs::read_to_string(repo_root().join("robocheck/fixtures/safe_shooter.rs")).unwrap();
        let a = assemble(&parse(&src).unwrap());
        let dir = temp_dir(name);
        materialize(&dir, &a.strategy_rs, &repo_root()).unwrap();
        (dir, a.strategy_rs)
    }

    #[test]
    fn battle_reports_the_hash_of_strategy_rs() {
        let (dir, strategy) = ready("battle-hash");
        let r = run_battle(&dir, &plain_target_dir(), "SittingDuck", 2000, &dir.join("recording.json")).unwrap();
        assert_eq!(r.artifact_hash, sha256_hex(strategy.as_bytes()));
        assert!(r.turns > 0);
        let rec = std::fs::read_to_string(dir.join("recording.json")).unwrap();
        assert!(rec.starts_with(r#"{"format":"roboverus-recording""#));
        assert_eq!(r.recording_hash, sha256_hex(rec.as_bytes()));
    }

    #[test]
    fn battle_is_deterministic() {
        let (dir, _) = ready("battle-det");
        let t = plain_target_dir();
        let a = run_battle(&dir, &t, "SittingDuck", 500, &dir.join("a.json")).unwrap();
        let b = run_battle(&dir, &t, "SittingDuck", 500, &dir.join("b.json")).unwrap();
        assert_eq!(a.recording_hash, b.recording_hash);
    }

    #[test]
    fn unknown_opponent_is_an_error() {
        let (dir, _) = ready("battle-unknown");
        let e = run_battle(&dir, &plain_target_dir(), "Nobody", 100, &dir.join("r.json")).unwrap_err();
        assert!(e.contains("SittingDuck"), "{e}");
    }

    #[test]
    fn hash_mismatch_is_flagged() {
        let e = check_same_artifact("aa", "bb").unwrap_err();
        assert!(e.contains("aa") && e.contains("bb"));
        assert_eq!(check_same_artifact("aa", "aa"), Ok(()));
    }
}
