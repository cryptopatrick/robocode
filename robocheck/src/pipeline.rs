//! The whole run: validate → assemble → build → verify → battle → evidence.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::battle::{Arena, BattleError, check_same_artifact, run_battle};
use crate::candidate::{parse, validate};
use crate::compile::{BuildOutcome, build};
use crate::evidence::{
    BattleInfo, Build, CandidateInfo, Evidence, Validation, Verification,
    rfc3339, toolchain,
};
use crate::template::{Spec, assemble, reference_spec, sha256_hex, spec_hash};
use crate::verify::{
    CargoVerus, PropertyResult, PropertyStatus, verify_properties,
};
use crate::workspace::{
    materialize, plain_target_dir, repo_root, verus_target_dir,
};

/// Options for one run.
#[derive(Debug, Clone)]
pub struct Options {
    pub candidate: PathBuf,
    pub opponent: String,
    pub turns: u64,
    /// Output folder; `None` = `target/robocheck/runs/<run_id>/`.
    pub out: Option<PathBuf>,
    pub verus_dir: PathBuf,
    /// Per Verus run.
    pub timeout: Duration,
    /// Wall-clock limit for the battle.
    pub battle_timeout: Duration,
}

fn all(
    spec: &Spec,
    status: PropertyStatus,
    note: &str,
) -> Vec<PropertyResult> {
    spec.rules
        .iter()
        .map(|(id, name)| PropertyResult {
            id: id.clone(),
            name: name.clone(),
            status,
            note: note.to_string(),
            failed_at: None,
        })
        .collect()
}

/// The arenas every candidate fights in, in report order.
pub const ARENAS: [Arena; 2] = [Arena::Open, Arena::Urban];

fn skipped_battle(arena: Arena, opponent: &str) -> BattleInfo {
    BattleInfo {
        arena: arena.name(),
        status: "skipped",
        opponent: opponent.to_string(),
        turns: None,
        winner: None,
        artifact_hash: None,
        same_artifact: None,
        recording_file: None,
        recording_sha256: None,
        decisions_file: None,
        error: None,
    }
}

/// Battles in both arenas, skipped.
pub fn skipped_battles(opponent: &str) -> Vec<BattleInfo> {
    ARENAS.iter().map(|a| skipped_battle(*a, opponent)).collect()
}

/// Fights the built candidate crate in `build_dir` in both arenas, writing
/// `recording-<arena>.json` and `decisions-<arena>.jsonl` into `out`.
/// `verified_hash` is the hash of the `strategy.rs` that was verified.
pub fn fight_both(
    build_dir: &Path,
    out: &Path,
    verified_hash: &str,
    opponent: &str,
    turns: u64,
    timeout: Duration,
) -> Vec<BattleInfo> {
    ARENAS
        .iter()
        .map(|&arena| {
            let file = format!("recording-{}.json", arena.name());
            match run_battle(
                build_dir,
                &plain_target_dir(),
                opponent,
                arena,
                turns,
                &out.join(&file),
                timeout,
            ) {
                // A battle of some other code says nothing about the verified robot: drop it.
                Ok(r)
                    if check_same_artifact(
                        verified_hash,
                        &r.artifact_hash,
                    )
                    .is_err() =>
                {
                    let _ = std::fs::remove_file(out.join(&file));
                    let _ = std::fs::remove_file(&r.decisions_file);
                    BattleInfo {
                        status: "failed",
                        same_artifact: Some(false),
                        error: check_same_artifact(
                            verified_hash,
                            &r.artifact_hash,
                        )
                        .err(),
                        artifact_hash: Some(r.artifact_hash),
                        ..skipped_battle(arena, opponent)
                    }
                }
                Ok(r) => BattleInfo {
                    arena: arena.name(),
                    status: "ok",
                    opponent: opponent.to_string(),
                    turns: Some(r.turns),
                    winner: r.winner,
                    same_artifact: Some(
                        check_same_artifact(verified_hash, &r.artifact_hash)
                            .is_ok(),
                    ),
                    artifact_hash: Some(r.artifact_hash),
                    recording_file: Some(file),
                    recording_sha256: Some(r.recording_hash),
                    decisions_file: r
                        .decisions_file
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned()),
                    error: None,
                },
                Err(e) => BattleInfo {
                    status: if matches!(e, BattleError::TimedOut(_)) {
                        "timed_out"
                    } else {
                        "failed"
                    },
                    error: Some(e.to_string()),
                    ..skipped_battle(arena, opponent)
                },
            }
        })
        .collect()
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text)
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// Runs the pipeline and writes `evidence.json` (plus logs, `strategy.rs` and
/// the recording) to the output folder. `Err` = the tool itself could not run.
pub fn run(o: &Options) -> Result<(PathBuf, Evidence), String> {
    let source = std::fs::read_to_string(&o.candidate).map_err(|e| {
        format!("cannot read candidate {}: {e}", o.candidate.display())
    })?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let parsed = parse(&source);
    let problems = match &parsed {
        Ok(c) => validate(c).err().unwrap_or_default(),
        Err(p) => p.clone(),
    };
    let spec = reference_spec();
    let assembled = parsed
        .ok()
        .filter(|_| problems.is_empty())
        .map(|c| assemble(&c, &spec));
    let artifact_hash =
        assembled.as_ref().map(|a| sha256_hex(a.strategy_rs.as_bytes()));
    let tag = artifact_hash
        .as_deref()
        .map_or_else(|| sha256_hex(source.as_bytes()), str::to_string);
    let run_id = format!("{}-{now}", &tag[..12]);
    let out = o.out.clone().unwrap_or_else(|| {
        repo_root().join("target/robocheck/runs").join(&run_id)
    });
    std::fs::create_dir_all(&out)
        .map_err(|e| format!("cannot create {}: {e}", out.display()))?;

    let mut ev = Evidence {
        format: "robocheck-evidence",
        version: 2,
        run_id: run_id.clone(),
        created_at: rfc3339(now),
        candidate: CandidateInfo {
            path: o.candidate.display().to_string(),
            sha256: sha256_hex(source.as_bytes()),
        },
        artifact_hash: artifact_hash.clone(),
        spec_hash: spec_hash(&spec),
        validation: Validation { ok: problems.is_empty(), problems },
        build: Build { status: "skipped", log_file: None },
        verification: Verification {
            properties: all(
                &spec,
                PropertyStatus::NotChecked,
                "the candidate was refused before verification",
            ),
            log_files: Vec::new(),
        },
        battles: skipped_battles(&o.opponent),
        toolchain: toolchain(&repo_root(), &o.verus_dir),
    };

    if let (Some(a), Some(hash)) = (&assembled, &artifact_hash) {
        write(&out.join("strategy.rs"), &a.strategy_rs)?;
        // Separate crates for the battle and for verification: verification
        // rewrites strategy.rs (neutralized clauses), the battle must not see that.
        let work = repo_root().join("target/robocheck/work").join(&run_id);
        let (build_dir, verify_dir) =
            (work.join("build"), work.join("verify"));
        for d in [&build_dir, &verify_dir] {
            materialize(d, &a.strategy_rs, &repo_root())
                .map_err(|e| format!("cannot write {}: {e}", d.display()))?;
        }
        match build(&build_dir, &plain_target_dir()) {
            BuildOutcome::Failed { log } => {
                write(&out.join("build.log"), &log)?;
                ev.build = Build {
                    status: "failed",
                    log_file: Some("build.log".into()),
                };
                ev.verification.properties = all(
                    &spec,
                    PropertyStatus::BuildFailed,
                    "the candidate does not compile",
                );
            }
            BuildOutcome::Ok => {
                write(&out.join("build.log"), "ok\n")?;
                ev.build =
                    Build { status: "ok", log_file: Some("build.log".into()) };
                let mut verus = CargoVerus {
                    dir: verify_dir,
                    target_dir: verus_target_dir(),
                    verus_dir: o.verus_dir.clone(),
                    timeout: o.timeout,
                };
                let (props, logs) = verify_properties(a, &spec, &mut verus, 8);
                ev.verification.properties = props;
                for (i, log) in logs.iter().enumerate() {
                    let name = format!("verus-{}.log", i + 1);
                    write(&out.join(&name), log)?;
                    ev.verification.log_files.push(name);
                }
                ev.battles = fight_both(
                    &build_dir,
                    &out,
                    hash,
                    &o.opponent,
                    o.turns,
                    o.battle_timeout,
                );
            }
        }
    }
    let json = serde_json::to_string_pretty(&ev).map_err(|e| e.to_string())?;
    write(&out.join("evidence.json"), &json)?;
    Ok((out, ev))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::parse;
    use crate::template::{assemble, reference_spec};
    use crate::workspace::{
        materialize,
        tests::{shared_target, temp_dir},
    };

    /// Review finding 3: a battle whose binary isn't the verified code is not a battle of that code.
    #[test]
    fn battle_on_different_code_is_failed() {
        let _guard = shared_target();
        let src = std::fs::read_to_string(
            repo_root().join("robocheck/fixtures/safe_shooter.rs"),
        )
        .unwrap();
        let a = assemble(&parse(&src).unwrap(), &reference_spec());
        let dir = temp_dir("pipeline-mismatch");
        materialize(&dir, &a.strategy_rs, &repo_root()).unwrap();
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let battles = fight_both(
            &dir,
            &out,
            "not-the-verified-hash",
            "SittingDuck",
            200,
            Duration::from_secs(120),
        );
        for b in &battles {
            assert_eq!(b.status, "failed", "{b:?}");
            assert_eq!(b.same_artifact, Some(false));
            assert!(
                b.recording_file.is_none() && b.decisions_file.is_none(),
                "{b:?}"
            );
            assert!(
                b.error.as_deref().is_some_and(|e| e.contains("differs")),
                "{b:?}"
            );
        }
    }
}
