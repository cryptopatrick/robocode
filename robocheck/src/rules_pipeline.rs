//! `robocheck rules`: plain-English rules → the AI's spec → the AI's robot →
//! Verus → open and urban battles → `run.json` (plus evidence, code, logs,
//! recordings and decision logs) in one output folder.
//!
//! The AI writes the robot once; when Verus rejects it, the run reports where
//! and the robot still fights.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use crate::ai::{AiCost, AiError, ExplainInput, RuleOutcome, explain, formalise, generate, read_back};
use crate::candidate::{parse, validate as validate_candidate};
use crate::compile::{BuildOutcome, build};
use crate::evidence::{Build, CandidateInfo, Evidence, Validation, Verification, rfc3339, toolchain};
use crate::llm::{Llm, LlmError, MODEL};
use crate::page::{
    ArenaViolations, BattleEntry, Battles, Broken, Code, DeletedStarter, ExplanationEntry, NotFormalised, RuleEntry, RunPage,
};
use crate::pipeline::{fight_both, skipped_battles};
use crate::rulespec::{Clause, Rule, reference_clauses, starter_rules};
use crate::template::{assemble, generated_spec, sha256_hex, spec_hash};
use crate::verify::{CargoVerus, PropertyResult, PropertyStatus, Verifier, verify_properties};
use crate::violations::{Violation, find_violations};
use crate::workspace::{materialize, plain_target_dir, repo_root};

/// Most visitor rules in one run (ids `U1`..`U5`).
pub const MAX_VISITOR_RULES: usize = 5;
/// Longest rule text, in characters.
pub const MAX_RULE_CHARS: usize = 280;
const STARTER_IDS: [&str; 3] = ["S1", "S2", "S3"];

/// Options for one `robocheck rules` run.
#[derive(Debug, Clone)]
pub struct RulesOptions {
    /// `{"rules": [{"id": "S1", "text": "…"}, …]}`.
    pub rules_file: PathBuf,
    pub opponent: String,
    pub turns: u64,
    /// Output folder; `None` = `target/robocheck/runs/<run_id>/`.
    pub out: Option<PathBuf>,
    pub battle_timeout: Duration,
    /// `live` or `stored`, recorded in `run.json`.
    pub source: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RulesFile {
    rules: Vec<Rule>,
}

/// Reads and checks a rules file: ids `S1`–`S3` and `U1`–`U5`, each at most
/// once; non-blank text of at most 280 characters.
pub fn read_rules(path: &Path) -> Result<Vec<Rule>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let file: RulesFile = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut seen = BTreeSet::new();
    let visitor_ids: Vec<String> = (1..=MAX_VISITOR_RULES).map(|i| format!("U{i}")).collect();
    for r in &file.rules {
        if !STARTER_IDS.contains(&r.id.as_str()) && !visitor_ids.contains(&r.id) {
            return Err(format!("rule id {} is not one of S1–S3 or U1–U{MAX_VISITOR_RULES}", r.id));
        }
        if !seen.insert(r.id.clone()) {
            return Err(format!("rule {} appears twice", r.id));
        }
        if r.text.trim().is_empty() {
            return Err(format!("rule {} is empty", r.id));
        }
        if r.text.chars().count() > MAX_RULE_CHARS {
            return Err(format!("rule {} is longer than {MAX_RULE_CHARS} characters", r.id));
        }
    }
    Ok(file.rules.into_iter().map(|r| Rule { id: r.id, text: r.text.trim().to_string() }).collect())
}

fn status_name(s: PropertyStatus) -> &'static str {
    match s {
        PropertyStatus::Proved => "proved",
        PropertyStatus::Rejected => "rejected",
        PropertyStatus::Unknown | PropertyStatus::TimedOut => "unknown",
        PropertyStatus::NotChecked | PropertyStatus::BuildFailed => "not_checked",
    }
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn ai_failure(step: &str, e: &AiError) -> String {
    match e {
        AiError::Llm(LlmError::Refused(c)) => {
            format!("the model declined to {step} ({})", c.as_deref().unwrap_or("no category"))
        }
        e => format!("could not {step}: {e}"),
    }
}

/// What a run has found so far; turned into `run.json` at any stopping point.
struct State {
    rules: Vec<Rule>,
    outcomes: Vec<RuleOutcome>,
    read_backs: Vec<(String, String)>,
    clauses: Vec<(String, Vec<String>)>,
    verdicts: Vec<PropertyResult>,
    violations: Vec<(String, Vec<Violation>)>,
    whys: Vec<(String, String)>,
    code: Code,
    explanation: Option<ExplanationEntry>,
    battles: Battles,
    same_artifact: bool,
}

fn broken(violations: &[(String, Vec<Violation>)], arena: &str, rule: &str) -> Option<Broken> {
    violations
        .iter()
        .find(|(a, _)| a == arena)
        .and_then(|(_, vs)| vs.iter().find(|v| v.rule == rule))
        .map(|v| Broken { first_turn: v.first_turn, turns: v.turns })
}

fn arena_violations(violations: &[(String, Vec<Violation>)], rule: &str) -> ArenaViolations {
    ArenaViolations { open: broken(violations, "open", rule), urban: broken(violations, "urban", rule) }
}

impl State {
    fn entries(&self) -> Vec<RuleEntry> {
        self.rules
            .iter()
            .map(|r| {
                let outcome = self.outcomes.iter().find(|o| o.id() == r.id);
                let verdict = self.verdicts.iter().find(|v| v.id == r.id);
                let violations = arena_violations(&self.violations, &r.id);
                let not_formalised = match outcome {
                    Some(RuleOutcome::NotFormalised { kind, reason, .. }) => {
                        Some(NotFormalised { kind: kind.clone(), reason: reason.clone() })
                    }
                    _ => None,
                };
                let status = if not_formalised.is_some() {
                    "not_formalised"
                } else {
                    verdict.map_or("not_checked", |v| status_name(v.status))
                };
                let triggered = match status {
                    "rejected" if violations.open.is_some() || violations.urban.is_some() => "yes",
                    "rejected" => "rejected_not_triggered",
                    _ => "n/a",
                };
                RuleEntry {
                    id: r.id.clone(),
                    text: r.text.clone(),
                    status,
                    read_back: self.read_backs.iter().find(|(id, _)| id == &r.id).map(|(_, s)| s.clone()),
                    clauses: self.clauses.iter().find(|(id, _)| id == &r.id).map(|(_, c)| c.clone()).unwrap_or_default(),
                    not_formalised,
                    failed_at: verdict.and_then(|v| v.failed_at),
                    why: self.whys.iter().find(|(id, _)| id == &r.id).map(|(_, w)| w.clone()),
                    violations,
                    triggered,
                }
            })
            .collect()
    }

    fn deleted_starters(&self) -> Vec<DeletedStarter> {
        starter_rules()
            .into_iter()
            .filter(|s| !self.rules.iter().any(|r| r.id == s.id))
            .map(|s| DeletedStarter { violations: arena_violations(&self.violations, &s.id), id: s.id, text: s.text })
            .collect()
    }

    fn outcome(&self) -> &'static str {
        if self.clauses.iter().all(|(_, c)| c.is_empty()) {
            return "no_rules_in_force";
        }
        let has = |s: PropertyStatus| self.verdicts.iter().any(|v| v.status == s);
        if has(PropertyStatus::Rejected) {
            "rejected"
        } else if has(PropertyStatus::TimedOut) {
            "timed_out"
        } else if has(PropertyStatus::Unknown) {
            "unknown"
        } else {
            "proved"
        }
    }
}

struct Output<'a> {
    out: &'a Path,
    run_id: &'a str,
    created_at: String,
    source: &'a str,
}

fn finish(o: &Output, s: &State, cost: &AiCost, outcome: &'static str, note: Option<String>) -> Result<RunPage, String> {
    let page = RunPage {
        format: "robocheck-run",
        version: 1,
        run_id: o.run_id.to_string(),
        created_at: o.created_at.clone(),
        source: o.source.to_string(),
        model: MODEL,
        robocode: toolchain(&repo_root(), &CargoVerus::default_verus_dir()).robocode,
        robocheck_arena: "0.1.0",
        cost_usd: (cost.usd * 1e6).round() / 1e6,
        outcome,
        outcome_note: note,
        rules: s.entries(),
        deleted_starters: s.deleted_starters(),
        code: s.code.clone(),
        explanation: s.explanation.clone(),
        battles: s.battles.clone(),
        same_artifact: s.same_artifact,
        evidence: "evidence.json",
    };
    let json = serde_json::to_string_pretty(&page).map_err(|e| e.to_string())?;
    write(&o.out.join("run.json"), &json)?;
    Ok(page)
}

/// Runs the whole pipeline. `make_verifier` gets the verification crate's
/// folder and returns the verifier to use (real Verus: [`CargoVerus`]).
/// `Err` = the rules were refused or the tool itself could not run; any
/// failure after that is reported in `run.json` as the run's outcome.
pub fn run_rules(
    o: &RulesOptions,
    llm: &mut dyn Llm,
    make_verifier: &mut dyn FnMut(&Path) -> Box<dyn Verifier>,
) -> Result<(PathBuf, RunPage), String> {
    let rules = read_rules(&o.rules_file)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let rules_json = serde_json::to_string(&rules.iter().map(|r| (&r.id, &r.text)).collect::<Vec<_>>()).unwrap_or_default();
    let run_id = format!("{}-{now}", &sha256_hex(rules_json.as_bytes())[..12]);
    let out = o.out.clone().unwrap_or_else(|| repo_root().join("target/robocheck/runs").join(&run_id));
    std::fs::create_dir_all(&out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
    let output = Output { out: &out, run_id: &run_id, created_at: rfc3339(now), source: &o.source };
    let mut cost = AiCost::default();
    let mut s = State {
        rules: rules.clone(),
        outcomes: Vec::new(),
        read_backs: Vec::new(),
        clauses: Vec::new(),
        verdicts: Vec::new(),
        violations: Vec::new(),
        whys: Vec::new(),
        code: Code::default(),
        explanation: None,
        battles: Battles::default(),
        same_artifact: false,
    };

    // 1. Formalise (no call when there are no rules).
    if !rules.is_empty() {
        match formalise(llm, &rules, &mut cost) {
            Ok(outcomes) => s.outcomes = outcomes,
            Err(e) => {
                let page = finish(&output, &s, &cost, "formalise_failed", Some(ai_failure("formalise the rules", &e)))?;
                return Ok((out, page));
            }
        }
    }
    let formalised: Vec<(String, String, Vec<Clause>)> = rules
        .iter()
        .map(|r| {
            let clauses = match s.outcomes.iter().find(|x| x.id() == r.id) {
                Some(RuleOutcome::Formalised { clauses, .. }) => clauses.clone(),
                _ => Vec::new(),
            };
            (r.id.clone(), r.text.clone(), clauses)
        })
        .collect();
    let spec = generated_spec(&formalised);
    s.clauses = rules
        .iter()
        .map(|r| (r.id.clone(), spec.clauses.iter().filter(|c| c.rule == r.id).map(|c| c.verus.clone()).collect()))
        .collect();

    // 2. Read back (a failure here only leaves the read-backs empty).
    s.read_backs = read_back(llm, &spec, &mut cost).unwrap_or_default();

    // 3. Generate the robot, once.
    let candidate_src = match generate(llm, &spec, &mut cost) {
        Ok(src) => src,
        Err(e) => {
            let page = finish(&output, &s, &cost, "generate_failed", Some(ai_failure("write the robot", &e)))?;
            return Ok((out, page));
        }
    };
    write(&out.join("candidate.rs"), &candidate_src)?;
    s.code.candidate = Some(candidate_src.clone());
    let parsed = parse(&candidate_src);
    let problems = match &parsed {
        Ok(c) => validate_candidate(c).err().unwrap_or_default(),
        Err(p) => p.clone(),
    };
    let mut ev = Evidence {
        format: "robocheck-evidence",
        version: 2,
        run_id: run_id.clone(),
        created_at: rfc3339(now),
        candidate: CandidateInfo { path: "candidate.rs".into(), sha256: sha256_hex(candidate_src.as_bytes()) },
        artifact_hash: None,
        spec_hash: spec_hash(&spec),
        validation: Validation { ok: problems.is_empty(), problems: problems.clone() },
        build: Build { status: "skipped", log_file: None },
        verification: Verification { properties: Vec::new(), log_files: Vec::new() },
        battles: skipped_battles(&o.opponent),
        toolchain: toolchain(&repo_root(), &CargoVerus::default_verus_dir()),
    };
    let write_evidence = |ev: &Evidence| -> Result<(), String> {
        write(&out.join("evidence.json"), &serde_json::to_string_pretty(ev).map_err(|e| e.to_string())?)
    };
    let candidate = match parsed {
        Ok(c) if problems.is_empty() => c,
        _ => {
            write_evidence(&ev)?;
            let note = format!("the AI's robot was refused before it ran: {}", problems.join("; "));
            let page = finish(&output, &s, &cost, "refused", Some(note))?;
            return Ok((out, page));
        }
    };

    // 4. Assemble, build, verify.
    let a = assemble(&candidate, &spec);
    let hash = sha256_hex(a.strategy_rs.as_bytes());
    ev.artifact_hash = Some(hash.clone());
    write(&out.join("strategy.rs"), &a.strategy_rs)?;
    s.code.strategy_rs = Some(a.strategy_rs.clone());
    let work = repo_root().join("target/robocheck/work").join(&run_id);
    let (build_dir, verify_dir) = (work.join("build"), work.join("verify"));
    for d in [&build_dir, &verify_dir] {
        materialize(d, &a.strategy_rs, &repo_root()).map_err(|e| format!("cannot write {}: {e}", d.display()))?;
    }
    if let BuildOutcome::Failed { log } = build(&build_dir, &plain_target_dir()) {
        write(&out.join("build.log"), &log)?;
        ev.build = Build { status: "failed", log_file: Some("build.log".into()) };
        write_evidence(&ev)?;
        let page = finish(&output, &s, &cost, "build_failed", Some("the AI's robot does not compile".into()))?;
        return Ok((out, page));
    }
    write(&out.join("build.log"), "ok\n")?;
    ev.build = Build { status: "ok", log_file: Some("build.log".into()) };
    let mut verifier = make_verifier(&verify_dir);
    let (verdicts, logs) = verify_properties(&a, &spec, verifier.as_mut(), 8);
    for (i, log) in logs.iter().enumerate() {
        let name = format!("verus-{}.log", i + 1);
        write(&out.join(&name), log)?;
        ev.verification.log_files.push(name);
    }
    ev.verification.properties = verdicts.clone();
    s.verdicts = verdicts;

    // 5. Battles: the same code, rejected or not, in both arenas.
    ev.battles = fight_both(&build_dir, &out, &hash, &o.opponent, o.turns, o.battle_timeout);
    let checked: Vec<(String, Vec<Clause>)> = formalised
        .iter()
        .filter(|(_, _, c)| !c.is_empty())
        .map(|(id, _, c)| (id.clone(), c.clone()))
        .chain(
            STARTER_IDS
                .iter()
                .filter(|id| !rules.iter().any(|r| r.id == **id))
                .map(|id| (id.to_string(), reference_clauses(id))),
        )
        .collect();
    for b in &ev.battles {
        let entry = BattleEntry {
            status: b.status,
            recording: b.recording_file.clone(),
            decisions: b.decisions_file.clone(),
            winner: b.winner.clone(),
            turns: b.turns,
            error: b.error.clone(),
        };
        if let Some(file) = &b.decisions_file {
            let log = std::fs::read_to_string(out.join(file)).map_err(|e| format!("cannot read {file}: {e}"))?;
            s.violations.push((b.arena.to_string(), find_violations(&log, &checked)?));
        }
        match b.arena {
            "open" => s.battles.open = Some(entry),
            _ => s.battles.urban = Some(entry),
        }
    }
    s.same_artifact = !ev.battles.is_empty() && ev.battles.iter().all(|b| b.same_artifact == Some(true));
    write_evidence(&ev)?;

    // 6. Explain (a failure here only leaves the explanation out).
    let input = ExplainInput { spec: &spec, rules: &rules, candidate: &candidate_src, verdicts: &s.verdicts, violations: &s.violations };
    if let Ok(e) = explain(llm, &input, &mut cost) {
        s.whys = e.per_rule;
        s.explanation = Some(ExplanationEntry { headline: e.headline, summary: e.summary });
    }
    let outcome = s.outcome();
    let page = finish(&output, &s, &cost, outcome, None)?;
    Ok((out, page))
}
