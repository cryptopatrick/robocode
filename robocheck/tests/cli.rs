//! The `robocheck run` command, for the paths that never reach Verus.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("fixtures/{name}.rs"))
}

/// Runs `robocheck run` into a fresh dir; returns (exit code, evidence if written, out dir).
pub fn run(args: &[&str], tag: &str) -> (i32, Option<Value>, PathBuf) {
    let out = std::env::temp_dir().join(format!("robocheck-cli-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let status = Command::new(env!("CARGO_BIN_EXE_robocheck"))
        .args(args)
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();
    let ev = std::fs::read_to_string(out.join("evidence.json")).ok().map(|s| serde_json::from_str(&s).unwrap());
    (status.code().unwrap_or(-1), ev, out)
}

fn statuses(ev: &Value) -> Vec<String> {
    ev["verification"]["properties"].as_array().unwrap().iter().map(|p| p["status"].as_str().unwrap().to_string()).collect()
}

#[test]
fn cheat_is_refused_before_anything_runs() {
    let (code, ev, out) = run(&["run", fixture("cheat_assume").to_str().unwrap()], "cheat");
    let ev = ev.expect("evidence.json");
    assert_eq!(code, 0);
    assert_eq!(ev["validation"]["ok"], false);
    assert!(ev["validation"]["problems"].to_string().contains("assume("));
    assert_eq!(statuses(&ev), vec!["not_checked"; 7]);
    assert_eq!(ev["build"]["status"], "skipped");
    assert_eq!(ev["battle"]["status"], "skipped");
    assert!(!out.join("build.log").exists());
}

#[test]
fn type_error_is_build_failed_and_battle_skipped() {
    let (code, ev, out) = run(&["run", fixture("type_error").to_str().unwrap()], "typeerr");
    let ev = ev.expect("evidence.json");
    assert_eq!(code, 0);
    assert_eq!(ev["build"]["status"], "failed");
    assert!(std::fs::read_to_string(out.join("build.log")).unwrap().contains("mismatched types"));
    assert_eq!(statuses(&ev), vec!["build_failed"; 7]);
    assert_eq!(ev["battle"]["status"], "skipped");
    assert_eq!(ev["format"], "robocheck-evidence");
    assert_eq!(ev["spec_hash"].as_str().unwrap().len(), 64);
}

#[test]
fn missing_candidate_is_a_tool_error() {
    let (code, ev, _) = run(&["run", "/no/such/file.rs"], "missing");
    assert_eq!(code, 2);
    assert!(ev.is_none());
}
