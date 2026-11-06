//! Battle stage: the candidate crate's battle binary fights a built-in robot.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::verify::kill_group;

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

/// Why a battle produced no result.
#[derive(Debug, Clone, PartialEq)]
pub enum BattleError {
    /// The battle binary ran past its time limit and was killed.
    TimedOut(Duration),
    Failed(String),
}

impl std::fmt::Display for BattleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BattleError::TimedOut(d) => write!(f, "the battle exceeded its time limit of {} s", d.as_secs()),
            BattleError::Failed(e) => f.write_str(e),
        }
    }
}

/// Builds the candidate crate in `dir` and runs its battle binary against
/// `opponent`, writing the recording to `recording_out`.
pub fn run_battle(
    dir: &Path,
    target_dir: &Path,
    opponent: &str,
    max_turns: u64,
    recording_out: &Path,
    timeout: Duration,
) -> Result<BattleResult, BattleError> {
    let fail = |e: String| BattleError::Failed(e);
    let strategy = std::fs::read_to_string(dir.join("src/strategy.rs")).map_err(|e| fail(e.to_string()))?;
    // Named after this exact strategy.rs: if the file was changed after
    // materialize, the binary is not found rather than silently rebuilt.
    let (_, bin) = crate_and_bin(&strategy);
    if let BuildOutcome::Failed { log } = build(dir, target_dir) {
        return Err(fail(format!("battle build failed:\n{log}")));
    }
    let exe = target_dir.join("release").join(&bin);
    let stdout_path = recording_out.with_extension("stdout");
    let stderr_path = recording_out.with_extension("stderr");
    let file = |p: &Path| std::fs::File::create(p).map_err(|e| fail(e.to_string()));
    let mut cmd = Command::new(&exe);
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    let mut child = cmd
        .args(["--opponent", opponent, "--turns", &max_turns.to_string(), "--out"])
        .arg(recording_out)
        .stdout(file(&stdout_path)?)
        .stderr(file(&stderr_path)?)
        .spawn()
        .map_err(|e| fail(format!("cannot run {}: {e}", exe.display())))?;
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|e| fail(e.to_string()))? {
            break s;
        }
        if start.elapsed() > timeout {
            kill_group(child.id());
            let _ = child.kill();
            let _ = child.wait();
            return Err(BattleError::TimedOut(timeout));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let read = |p: &Path| std::fs::read(p).unwrap_or_default();
    let (stdout, stderr) = (read(&stdout_path), read(&stderr_path));
    let _ = (std::fs::remove_file(&stdout_path), std::fs::remove_file(&stderr_path));
    if !status.success() {
        return Err(fail(String::from_utf8_lossy(&stderr).trim().to_string()));
    }
    let line: Line = serde_json::from_slice(&stdout).map_err(|e| fail(format!("unexpected battle output: {e}")))?;
    let rec = std::fs::read(recording_out).map_err(|e| fail(e.to_string()))?;
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
    use crate::workspace::{materialize, plain_target_dir, repo_root, tests::{shared_target, temp_dir}};

    const LIMIT: std::time::Duration = std::time::Duration::from_secs(120);

    fn ready(name: &str) -> (std::path::PathBuf, String) {
        ready_fixture(name, "safe_shooter")
    }

    fn ready_fixture(name: &str, fixture: &str) -> (std::path::PathBuf, String) {
        let src = std::fs::read_to_string(repo_root().join(format!("robocheck/fixtures/{fixture}.rs"))).unwrap();
        let a = assemble(&parse(&src).unwrap());
        let dir = temp_dir(name);
        materialize(&dir, &a.strategy_rs, &repo_root()).unwrap();
        (dir, a.strategy_rs)
    }

    #[test]
    fn battle_reports_the_hash_of_strategy_rs() {
        let _guard = shared_target();
        let (dir, strategy) = ready("battle-hash");
        let r = run_battle(&dir, &plain_target_dir(), "SittingDuck", 2000, &dir.join("recording.json"), LIMIT).unwrap();
        assert_eq!(r.artifact_hash, sha256_hex(strategy.as_bytes()));
        assert!(r.turns > 0);
        let rec = std::fs::read_to_string(dir.join("recording.json")).unwrap();
        assert!(rec.starts_with(r#"{"format":"roboverus-recording""#));
        assert_eq!(r.recording_hash, sha256_hex(rec.as_bytes()));
    }

    #[test]
    fn battle_is_deterministic() {
        let _guard = shared_target();
        let (dir, _) = ready("battle-det");
        let t = plain_target_dir();
        let a = run_battle(&dir, &t, "SittingDuck", 500, &dir.join("a.json"), LIMIT).unwrap();
        let b = run_battle(&dir, &t, "SittingDuck", 500, &dir.join("b.json"), LIMIT).unwrap();
        assert_eq!(a.recording_hash, b.recording_hash);
    }

    #[test]
    fn unknown_opponent_is_an_error() {
        let _guard = shared_target();
        let (dir, _) = ready("battle-unknown");
        let e = run_battle(&dir, &plain_target_dir(), "Nobody", 100, &dir.join("r.json"), LIMIT).unwrap_err();
        assert!(e.to_string().contains("SittingDuck"), "{e}");
    }

    /// Review finding 3: a candidate that hangs must not hang robocheck.
    #[test]
    fn hung_battle_times_out() {
        let _guard = shared_target();
        let (dir, _) = ready_fixture("battle-hang", "hangs");
        let t0 = std::time::Instant::now();
        let e = run_battle(&dir, &plain_target_dir(), "SittingDuck", 100, &dir.join("r.json"), std::time::Duration::from_secs(3))
            .unwrap_err();
        assert!(matches!(e, BattleError::TimedOut(_)), "{e}");
        assert!(t0.elapsed() < std::time::Duration::from_secs(60));
    }

    /// Review finding 6: a strategy.rs changed after materialize is never battled as the verified one.
    #[test]
    fn changed_strategy_is_not_battled() {
        let (dir, strategy) = ready("battle-changed");
        std::fs::write(dir.join("src/strategy.rs"), strategy.replace("Power(1000)", "Power(2000)")).unwrap();
        // Its own target dir: this test deliberately builds a modified file
        // under the original crate name.
        let target = std::env::temp_dir().join(format!("robocheck-test-changed-target-{}", std::process::id()));
        assert!(run_battle(&dir, &target, "SittingDuck", 100, &dir.join("r.json"), LIMIT).is_err());
    }

    #[test]
    fn hash_mismatch_is_flagged() {
        let e = check_same_artifact("aa", "bb").unwrap_err();
        assert!(e.contains("aa") && e.contains("bb"));
        assert_eq!(check_same_artifact("aa", "aa"), Ok(()));
    }
}
