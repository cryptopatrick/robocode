//! `robocheck rules`: rules → AI spec → AI robot → Verus → two battles → run.json.
//! The AI is scripted and Verus is faked (it rejects the clauses of chosen rules);
//! the candidate crates are really built and the battles really fought.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Mutex;
use std::time::Duration;

use robocheck::llm::{LlmError, ScriptedLlm};
use robocheck::rules_pipeline::{RulesOptions, run_rules};
use robocheck::rulespec::reference_clauses;
use robocheck::verify::{Verifier, VerusRun};
use serde_json::{Value, json};

/// Candidate builds share one target dir; run these tests one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

/// Rejects the clauses of the given rules; counts its runs.
struct FakeVerus {
    reject: Vec<String>,
    runs: Rc<Cell<u32>>,
}

impl Verifier for FakeVerus {
    fn run(&mut self, strategy_rs: &str) -> Result<VerusRun, String> {
        self.runs.set(self.runs.get() + 1);
        let failing: Vec<usize> = strategy_rs
            .lines()
            .enumerate()
            .filter(|(_, l)| {
                self.reject.iter().any(|id| l.ends_with(&format!("// {id}")))
                    && !l.trim_start().starts_with("true,")
            })
            .map(|(i, _)| i + 1)
            .collect();
        if failing.is_empty() {
            return Ok(VerusRun {
                success: true,
                timed_out: false,
                log: "verification results:: 9 verified, 0 errors".into(),
            });
        }
        let log: String = failing
            .iter()
            .map(|l| format!("error: postcondition not satisfied\n  --> src/strategy.rs:{l}:9\n   |\n...\n77 |     Action {{ }}\n   |     --- at the end of the function body\n\n"))
            .collect();
        Ok(VerusRun {
            success: false,
            timed_out: false,
            log: log + "verification results:: 8 verified, 1 errors\n",
        })
    }
}

fn fixture_reply(name: &str) -> String {
    let src = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("fixtures/{name}.rs")),
    )
    .unwrap();
    let (head, body) = src.split_once("// @@body\n").unwrap();
    let helpers = head.split_once("// @@helpers\n").unwrap().1;
    json!({ "helpers": helpers, "body": body }).to_string()
}

fn formalised(id: &str, clauses: Value) -> Value {
    json!({"id": id, "formalised": true, "clauses": clauses, "not_formalised_kind": "none", "reason": "", "conflicts_with": ""})
}

fn reference(id: &str) -> Value {
    serde_json::to_value(reference_clauses(id)).unwrap()
}

fn read_backs(ids: &[&str]) -> String {
    json!({"read_backs": ids.iter().map(|id| json!({"id": id, "sentence": format!("read back {id}")})).collect::<Vec<_>>()}).to_string()
}

fn explanation(rejected: &[&str]) -> String {
    json!({
        "headline": "headline",
        "summary": "summary",
        "per_rule": rejected.iter().map(|id| json!({"id": id, "why": format!("why {id}")})).collect::<Vec<_>>()
    })
    .to_string()
}

struct Run {
    page: Value,
    out: PathBuf,
    llm: ScriptedLlm,
    verus_runs: u32,
}

fn run(
    tag: &str,
    rules: Value,
    replies: Vec<Result<String, LlmError>>,
    reject: &[&str],
) -> Result<Run, String> {
    run_with(tag, rules, replies, reject, Duration::from_secs(120), false)
}

/// Verus that cannot be run at all (e.g. not installed).
struct NoVerus;
impl Verifier for NoVerus {
    fn run(&mut self, _: &str) -> Result<VerusRun, String> {
        Err("cargo-verus not found".into())
    }
}

fn run_with(
    tag: &str,
    rules: Value,
    replies: Vec<Result<String, LlmError>>,
    reject: &[&str],
    battle_timeout: Duration,
    no_verus: bool,
) -> Result<Run, String> {
    let dir = std::env::temp_dir()
        .join(format!("robocheck-rules-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let rules_file = dir.join("rules.json");
    std::fs::write(&rules_file, rules.to_string()).unwrap();
    let out = dir.join("out");
    let opts = RulesOptions {
        rules_file,
        opponent: "SittingDuck".into(),
        turns: 800,
        out: Some(out.clone()),
        battle_timeout,
        source: "live".into(),
    };
    let mut llm = ScriptedLlm::new(replies);
    let runs = Rc::new(Cell::new(0));
    let (reject, counter) = (
        reject.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        Rc::clone(&runs),
    );
    let mut make = move |_dir: &Path| -> Box<dyn Verifier> {
        if no_verus {
            Box::new(NoVerus)
        } else {
            Box::new(FakeVerus {
                reject: reject.clone(),
                runs: Rc::clone(&counter),
            })
        }
    };
    let (written, _) = run_rules(&opts, &mut llm, &mut make)?;
    assert_eq!(written, out);
    let page: Value = serde_json::from_str(
        &std::fs::read_to_string(out.join("run.json")).unwrap(),
    )
    .unwrap();
    Ok(Run { page, out, llm, verus_runs: runs.get() })
}

fn rule<'a>(page: &'a Value, id: &str) -> &'a Value {
    page["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == id)
        .unwrap_or_else(|| panic!("no rule {id}"))
}

fn starters() -> Value {
    json!({"rules": [
        {"id": "S1", "text": "Never fire a shot you can't afford."},
        {"id": "S2", "text": "Only fire at a robot you saw this turn, with the gun aimed at it."},
        {"id": "S3", "text": "Never drive toward a wall or building closer than 80 px."}
    ]})
}

fn formalise_all_starters() -> String {
    json!({"rules": [formalised("S1", reference("S1")), formalised("S2", reference("S2")), formalised("S3", reference("S3"))]}).to_string()
}

#[test]
fn writes_run_json_with_all_rules() {
    let _g = serial();
    let r = run(
        "all",
        starters(),
        vec![
            Ok(formalise_all_starters()),
            Ok(read_backs(&["S1", "S2", "S3"])),
            Ok(fixture_reply("safe_shooter")),
            Ok(explanation(&[])),
        ],
        &[],
    )
    .unwrap();
    let p = &r.page;
    assert_eq!(p["format"], "robocheck-run");
    assert_eq!(p["version"], 1);
    assert_eq!(p["source"], "live");
    assert_eq!(p["model"], "claude-sonnet-5-5");
    // The run records the robocode version of this repository.
    let root_manifest = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../Cargo.toml"
    ))
    .unwrap();
    let version = root_manifest
        .lines()
        .find_map(|l| l.strip_prefix("version = \""))
        .and_then(|v| v.strip_suffix('"'))
        .unwrap();
    assert_eq!(p["robocode"], version);
    assert_eq!(p["outcome"], "proved");
    assert_eq!(p["rules"].as_array().unwrap().len(), 3);
    for id in ["S1", "S2", "S3"] {
        let x = rule(p, id);
        assert_eq!(x["status"], "proved", "{x}");
        assert_eq!(x["read_back"], format!("read back {id}"));
        assert!(!x["clauses"].as_array().unwrap().is_empty());
        assert!(x["failed_at"].is_null());
    }
    assert_eq!(p["same_artifact"], true);
    for arena in ["open", "urban"] {
        let b = &p["battles"][arena];
        assert!(
            r.out.join(b["recording"].as_str().unwrap()).exists(),
            "{arena}"
        );
        assert!(
            r.out.join(b["decisions"].as_str().unwrap()).exists(),
            "{arena}"
        );
    }
    assert!(p["code"]["candidate"].as_str().unwrap().contains("// @@body"));
    assert!(p["code"]["strategy_rs"].as_str().unwrap().contains("// S1"));
    assert!(r.out.join("evidence.json").exists());
    assert_eq!(p["explanation"]["headline"], "headline");
}

#[test]
fn limits_checked_before_any_ai_call() {
    let _g = serial();
    let six: Vec<Value> = (1..=6)
        .map(|i| json!({"id": format!("U{i}"), "text": "Keep moving."}))
        .collect();
    let mut llm = ScriptedLlm::new(vec![]);
    let dir = std::env::temp_dir()
        .join(format!("robocheck-rules-limits-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, rules) in [
        ("six", json!({"rules": six})),
        ("long", json!({"rules": [{"id": "U1", "text": "x".repeat(281)}]})),
        ("badid", json!({"rules": [{"id": "S9", "text": "x"}]})),
        (
            "dup",
            json!({"rules": [{"id": "S1", "text": "x"}, {"id": "S1", "text": "y"}]}),
        ),
        ("empty", json!({"rules": [{"id": "U1", "text": "   "}]})),
    ] {
        let f = dir.join(format!("{name}.json"));
        std::fs::write(&f, rules.to_string()).unwrap();
        let opts = RulesOptions {
            rules_file: f,
            opponent: "SittingDuck".into(),
            turns: 100,
            out: Some(dir.join(name)),
            battle_timeout: Duration::from_secs(10),
            source: "live".into(),
        };
        let mut make = |_: &Path| -> Box<dyn Verifier> {
            panic!("no verification for {name}")
        };
        assert!(
            run_rules(&opts, &mut llm, &mut make).is_err(),
            "{name} should be refused"
        );
    }
    assert!(
        llm.requests.is_empty(),
        "an AI call was made before the limits were checked"
    );
}

#[test]
fn rejected_robot_still_fights() {
    let _g = serial();
    let r = run(
        "rejected",
        starters(),
        vec![
            Ok(formalise_all_starters()),
            Ok(read_backs(&["S1", "S2", "S3"])),
            Ok(fixture_reply("wall_blind")),
            Ok(explanation(&["S3"])),
        ],
        &["S3"],
    )
    .unwrap();
    let p = &r.page;
    assert_eq!(p["outcome"], "rejected");
    let s3 = rule(p, "S3");
    assert_eq!(s3["status"], "rejected");
    assert_eq!(s3["failed_at"]["code_line"], 77);
    assert!(s3["failed_at"]["clause_line"].as_u64().unwrap() > 0);
    assert_eq!(s3["why"], "why S3");
    // It still fought, and in town it drove into a building.
    assert!(p["battles"]["urban"]["turns"].as_u64().unwrap() > 0);
    assert!(
        s3["violations"]["urban"]["first_turn"].as_u64().is_some(),
        "{s3}"
    );
    assert_eq!(s3["triggered"], "yes");
    assert_eq!(rule(p, "S1")["status"], "proved");
    assert_eq!(rule(p, "S1")["triggered"], "n/a");
}

#[test]
fn rejected_not_triggered_is_reported() {
    let _g = serial();
    let r = run(
        "nottriggered",
        starters(),
        vec![
            Ok(formalise_all_starters()),
            Ok(read_backs(&["S1", "S2", "S3"])),
            Ok(fixture_reply("safe_shooter")),
            Ok(explanation(&["S1"])),
        ],
        &["S1"],
    )
    .unwrap();
    let s1 = rule(&r.page, "S1");
    assert_eq!(s1["status"], "rejected");
    assert!(
        s1["violations"]["open"].is_null()
            && s1["violations"]["urban"].is_null(),
        "{s1}"
    );
    assert_eq!(s1["triggered"], "rejected_not_triggered");
}

#[test]
fn deleted_starter_uses_reference_clauses() {
    let _g = serial();
    let rules = json!({"rules": [
        {"id": "S1", "text": "Never fire a shot you can't afford."},
        {"id": "S2", "text": "Only fire at a robot you saw this turn, with the gun aimed at it."}
    ]});
    let formalise = json!({"rules": [formalised("S1", reference("S1")), formalised("S2", reference("S2"))]}).to_string();
    let r = run(
        "deleted",
        rules,
        vec![
            Ok(formalise),
            Ok(read_backs(&["S1", "S2"])),
            Ok(fixture_reply("wall_blind")),
            Ok(explanation(&[])),
        ],
        &[],
    )
    .unwrap();
    let deleted = r.page["deleted_starters"].as_array().unwrap();
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0]["id"], "S3");
    assert!(
        deleted[0]["violations"]["urban"]["first_turn"].as_u64().is_some(),
        "{}",
        deleted[0]
    );
    assert_eq!(r.page["rules"].as_array().unwrap().len(), 2);
}

#[test]
fn no_rules_in_force() {
    let _g = serial();
    let r = run(
        "none",
        json!({"rules": []}),
        vec![Ok(fixture_reply("wall_blind")), Ok(explanation(&[]))],
        &[],
    )
    .unwrap();
    let p = &r.page;
    assert_eq!(p["outcome"], "no_rules_in_force");
    assert!(p["rules"].as_array().unwrap().is_empty());
    assert_eq!(p["deleted_starters"].as_array().unwrap().len(), 3);
    assert!(p["battles"]["open"]["turns"].as_u64().unwrap() > 0);
    // No formalise or read-back call: only generate and explain.
    assert_eq!(r.llm.requests.len(), 2);
}

#[test]
fn contradictory_rules_get_separate_verdicts() {
    let _g = serial();
    let rules = json!({"rules": [{"id": "U1", "text": "Only fire with a cool gun."}, {"id": "U2", "text": "Only fire with a hot gun."}]});
    let cool = json!([{"when": "fires", "require": {"pred": "gun_cool"}}]);
    let hot =
        json!([{"when": "fires", "require": {"not": {"pred": "gun_cool"}}}]);
    let formalise =
        json!({"rules": [formalised("U1", cool), formalised("U2", hot)]})
            .to_string();
    let r = run(
        "contra",
        rules,
        vec![
            Ok(formalise),
            Ok(read_backs(&["U1", "U2"])),
            Ok(fixture_reply("safe_shooter")),
            Ok(explanation(&["U2"])),
        ],
        &["U2"],
    )
    .unwrap();
    assert_eq!(rule(&r.page, "U1")["status"], "proved");
    assert_eq!(rule(&r.page, "U2")["status"], "rejected");
    assert_eq!(r.page["outcome"], "rejected");
    assert_eq!(r.page["deleted_starters"].as_array().unwrap().len(), 3);
}

#[test]
fn generate_called_exactly_once() {
    let _g = serial();
    let r = run(
        "once",
        starters(),
        vec![
            Ok(formalise_all_starters()),
            Ok(read_backs(&["S1", "S2", "S3"])),
            Ok(fixture_reply("wall_blind")),
            Ok(explanation(&["S3"])),
        ],
        &["S3"],
    )
    .unwrap();
    let generates = r
        .llm
        .requests
        .iter()
        .filter(|q| q.system.contains("brain of a robot tank"))
        .count();
    assert_eq!(generates, 1);
    assert_eq!(r.llm.requests.len(), 4);
}

#[test]
fn formalise_failure_ends_run_without_verus() {
    let _g = serial();
    let r = run("ffail", starters(), vec![Ok("{\"rules\": [".into())], &[])
        .unwrap();
    assert_eq!(r.page["outcome"], "formalise_failed");
    assert_eq!(r.verus_runs, 0);
    assert!(
        r.page["battles"]["open"].is_null()
            && r.page["battles"]["urban"].is_null()
    );
    assert_eq!(r.llm.requests.len(), 1);
    for id in ["S1", "S2", "S3"] {
        assert_eq!(rule(&r.page, id)["status"], "not_checked");
    }
    let r = run(
        "refused",
        starters(),
        vec![Err(LlmError::Refused {
            category: Some("cyber".into()),
            cost_usd: 0.0,
        })],
        &[],
    )
    .unwrap();
    assert_eq!(r.page["outcome"], "formalise_failed");
}

/// The same flow with the real Verus: the scripted AI's robot (wall_blind)
/// keeps S1 and S2 but drives into walls, so Verus rejects S3.
#[test]
#[ignore = "needs Verus"]
fn real_verus_rejects_wall_blind_on_s3() {
    use robocheck::verify::CargoVerus;
    let _g = serial();
    let dir = std::env::temp_dir()
        .join(format!("robocheck-rules-realverus-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let rules_file = dir.join("rules.json");
    std::fs::write(&rules_file, starters().to_string()).unwrap();
    let opts = RulesOptions {
        rules_file,
        opponent: "SittingDuck".into(),
        turns: 800,
        out: Some(dir.join("out")),
        battle_timeout: Duration::from_secs(120),
        source: "live".into(),
    };
    let mut llm = ScriptedLlm::new(vec![
        Ok(formalise_all_starters()),
        Ok(read_backs(&["S1", "S2", "S3"])),
        Ok(fixture_reply("wall_blind")),
        Ok(explanation(&["S3"])),
    ]);
    let mut make = |d: &Path| -> Box<dyn Verifier> {
        Box::new(CargoVerus {
            dir: d.to_path_buf(),
            target_dir: robocheck::workspace::verus_target_dir(),
            verus_dir: CargoVerus::default_verus_dir(),
            timeout: Duration::from_secs(300),
        })
    };
    let (_, page) = run_rules(&opts, &mut llm, &mut make).unwrap();
    let statuses: Vec<(&str, &str)> =
        page.rules.iter().map(|r| (r.id.as_str(), r.status)).collect();
    assert_eq!(
        statuses,
        [("S1", "proved"), ("S2", "proved"), ("S3", "rejected")]
    );
    let s3 = &page.rules[2];
    assert!(
        s3.failed_at.is_some_and(|f| f.code_line.is_some()),
        "{:?}",
        s3.failed_at
    );
    assert_eq!(s3.triggered, "yes");
    assert!(page.same_artifact);
}

/// Review finding 1: Verus that never ran must not make a run "proved".
#[test]
fn verus_unavailable_is_not_proved() {
    let _g = serial();
    let r = run_with(
        "noverus",
        starters(),
        vec![
            Ok(formalise_all_starters()),
            Ok(read_backs(&["S1", "S2", "S3"])),
            Ok(fixture_reply("safe_shooter")),
            Ok(explanation(&[])),
        ],
        &[],
        Duration::from_secs(120),
        true,
    )
    .unwrap();
    assert_eq!(r.page["outcome"], "not_checked");
    for id in ["S1", "S2", "S3"] {
        assert_eq!(rule(&r.page, id)["status"], "not_checked");
    }
}

/// Review finding 2: with no battle fought, a rejected rule is not "never triggered".
#[test]
fn rejected_rule_without_a_battle_is_not_fought() {
    let _g = serial();
    let r = run_with(
        "nobattle",
        starters(),
        vec![
            Ok(formalise_all_starters()),
            Ok(read_backs(&["S1", "S2", "S3"])),
            Ok(fixture_reply("hangs")),
            Ok(explanation(&["S1"])),
        ],
        &["S1"],
        Duration::from_secs(2),
        false,
    )
    .unwrap();
    assert_eq!(r.page["battles"]["open"]["status"], "timed_out");
    let s1 = rule(&r.page, "S1");
    assert_eq!(s1["status"], "rejected");
    assert_eq!(s1["triggered"], "not_fought");
}
