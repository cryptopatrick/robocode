use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn has_cargo_verus() -> bool {
    Command::new("cargo").args(["verus", "--help"]).output().map(|o| o.status.success()).unwrap_or(false)
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cargo-robocode-verify-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tool(args: &[&str], dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-robocode"))
        .args(args)
        .current_dir(dir)
        .env("ROBOCODE_BUILD_DIR", std::env::temp_dir().join("cargo-robocode-test-build"))
        .output()
        .unwrap()
}

const PROVED: &str = r#"
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
use robocode::strategy::{Action, Observation, Strategy};
use robocode::units::Power;

#[cfg_attr(verus_keep_ghost, verus_verify)]
#[cfg_attr(verus_keep_ghost, verus_spec(r =>
    ensures r.fire matches Some(p) ==> p.0 <= LIMIT,
))]
pub fn decide(obs: &Observation) -> Action {
    let _ = obs;
    Action { fire: Some(Power(1000)), ahead: None, turn_body: None, turn_gun: None, turn_radar: None }
}

#[derive(Default)]
pub struct Proved;

impl Strategy for Proved {
    fn name(&self) -> &str { "Proved" }
    fn decide(&mut self, obs: &Observation) -> Action { decide(obs) }
}

robocode::robot!(strategy Proved);
"#;

macro_rules! needs_verus {
    () => {
        if !has_cargo_verus() {
            eprintln!("skipped: cargo verus is not on PATH");
            return;
        }
    };
}

#[test]
fn verify_counts_zero_specs() {
    needs_verus!();
    let dir = temp_dir("zero");
    assert!(tool(&["new", "bot"], &dir).status.success());
    let out = tool(&["verify", "bot.rs"], &dir);
    let all = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "{all}");
    assert!(all.contains("0 specs checked in your robot"), "{all}");
}

#[test]
fn verify_proved_strategy() {
    needs_verus!();
    let dir = temp_dir("proved");
    std::fs::write(dir.join("proved.rs"), PROVED.replace("LIMIT", "3000")).unwrap();
    let out = tool(&["verify", "proved.rs"], &dir);
    let all = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "{all}");
    assert!(all.contains("Verified: 1 specs checked in your robot"), "{all}");
    // The same file still battles without Verus.
    let battle = tool(&["battle", "proved.rs", "--output", "b.json", "--no-open"], &dir);
    assert!(battle.status.success(), "{}", String::from_utf8_lossy(&battle.stderr));
}

#[test]
fn verify_failing_spec_exits_1() {
    needs_verus!();
    let dir = temp_dir("failing");
    std::fs::write(dir.join("proved.rs"), PROVED.replace("LIMIT", "500")).unwrap();
    let out = tool(&["verify", "proved.rs"], &dir);
    let all = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert_eq!(out.status.code(), Some(1), "{all}");
    assert!(all.contains("postcondition not satisfied"), "{all}");
}
