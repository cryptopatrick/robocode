//! The whole run: validate → assemble → build → verify → battle → evidence.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::battle::{check_same_artifact, run_battle};
use crate::candidate::{parse, validate};
use crate::compile::{BuildOutcome, build};
use crate::evidence::{BattleInfo, Build, CandidateInfo, Evidence, Validation, Verification, rfc3339, toolchain};
use crate::template::{PROPERTIES, assemble, sha256_hex, spec_hash};
use crate::verify::{CargoVerus, PropertyResult, PropertyStatus, verify_properties};
use crate::workspace::{materialize, plain_target_dir, repo_root, verus_target_dir};

/// Options for one run.
#[derive(Debug, Clone)]
pub struct Options {
    pub candidate: PathBuf,
    pub opponent: String,
    pub turns: u64,
    /// Output folder; `None` = `target/robocheck/runs/<run_id>/`.
    pub out: Option<PathBuf>,
    pub verus_dir: PathBuf,
    pub timeout: Duration,
}

fn all(status: PropertyStatus, note: &str) -> Vec<PropertyResult> {
    PROPERTIES
        .iter()
        .map(|p| PropertyResult { id: p.id, name: p.name, status, note: note.to_string() })
        .collect()
}

fn skipped_battle(opponent: &str) -> BattleInfo {
    BattleInfo {
        status: "skipped",
        opponent: opponent.to_string(),
        turns: None,
        winner: None,
        artifact_hash: None,
        same_artifact: None,
        recording_file: None,
        recording_sha256: None,
        error: None,
    }
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// Runs the pipeline and writes `evidence.json` (plus logs, `strategy.rs` and
/// the recording) to the output folder. `Err` = the tool itself could not run.
pub fn run(o: &Options) -> Result<(PathBuf, Evidence), String> {
    let source = std::fs::read_to_string(&o.candidate)
        .map_err(|e| format!("cannot read candidate {}: {e}", o.candidate.display()))?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let parsed = parse(&source);
    let problems = match &parsed {
        Ok(c) => validate(c).err().unwrap_or_default(),
        Err(p) => p.clone(),
    };
    let assembled = parsed.ok().filter(|_| problems.is_empty()).map(|c| assemble(&c));
    let artifact_hash = assembled.as_ref().map(|a| sha256_hex(a.strategy_rs.as_bytes()));
    let tag = artifact_hash.as_deref().map_or_else(|| sha256_hex(source.as_bytes()), str::to_string);
    let run_id = format!("{}-{now}", &tag[..12]);
    let out = o.out.clone().unwrap_or_else(|| repo_root().join("target/robocheck/runs").join(&run_id));
    std::fs::create_dir_all(&out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;

    let mut ev = Evidence {
        format: "robocheck-evidence",
        version: 1,
        run_id: run_id.clone(),
        created_at: rfc3339(now),
        candidate: CandidateInfo { path: o.candidate.display().to_string(), sha256: sha256_hex(source.as_bytes()) },
        artifact_hash: artifact_hash.clone(),
        spec_hash: spec_hash(),
        validation: Validation { ok: problems.is_empty(), problems },
        build: Build { status: "skipped", log_file: None },
        verification: Verification {
            properties: all(PropertyStatus::NotChecked, "the candidate was refused before verification"),
            log_files: Vec::new(),
        },
        battle: skipped_battle(&o.opponent),
        toolchain: toolchain(&repo_root(), &o.verus_dir),
    };

    if let (Some(a), Some(hash)) = (&assembled, &artifact_hash) {
        write(&out.join("strategy.rs"), &a.strategy_rs)?;
        // Separate crates for the battle and for verification: verification
        // rewrites strategy.rs (neutralized clauses), the battle must not see that.
        let work = repo_root().join("target/robocheck/work").join(&run_id);
        let (build_dir, verify_dir) = (work.join("build"), work.join("verify"));
        for d in [&build_dir, &verify_dir] {
            materialize(d, &a.strategy_rs, &repo_root()).map_err(|e| format!("cannot write {}: {e}", d.display()))?;
        }
        match build(&build_dir, &plain_target_dir()) {
            BuildOutcome::Failed { log } => {
                write(&out.join("build.log"), &log)?;
                ev.build = Build { status: "failed", log_file: Some("build.log".into()) };
                ev.verification.properties = all(PropertyStatus::BuildFailed, "the candidate does not compile");
            }
            BuildOutcome::Ok => {
                write(&out.join("build.log"), "ok\n")?;
                ev.build = Build { status: "ok", log_file: Some("build.log".into()) };
                let mut verus = CargoVerus {
                    dir: verify_dir,
                    target_dir: verus_target_dir(),
                    verus_dir: o.verus_dir.clone(),
                    timeout: o.timeout,
                };
                let (props, logs) = verify_properties(a, &mut verus, 8);
                ev.verification.properties = props;
                for (i, log) in logs.iter().enumerate() {
                    let name = format!("verus-{}.log", i + 1);
                    write(&out.join(&name), log)?;
                    ev.verification.log_files.push(name);
                }
                let recording = out.join("recording.json");
                ev.battle = match run_battle(&build_dir, &plain_target_dir(), &o.opponent, o.turns, &recording) {
                    Ok(r) => BattleInfo {
                        status: "ok",
                        opponent: o.opponent.clone(),
                        turns: Some(r.turns),
                        winner: r.winner,
                        same_artifact: Some(check_same_artifact(hash, &r.artifact_hash).is_ok()),
                        artifact_hash: Some(r.artifact_hash),
                        recording_file: Some("recording.json".into()),
                        recording_sha256: Some(r.recording_hash),
                        error: None,
                    },
                    Err(e) => BattleInfo { status: "failed", error: Some(e), ..skipped_battle(&o.opponent) },
                };
            }
        }
    }
    let json = serde_json::to_string_pretty(&ev).map_err(|e| e.to_string())?;
    write(&out.join("evidence.json"), &json)?;
    Ok((out, ev))
}
