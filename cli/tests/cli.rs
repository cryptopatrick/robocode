use std::path::PathBuf;
use std::process::{Command, Output};

fn tool(args: &[&str], dir: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-robocode")).args(args).current_dir(dir).output().unwrap()
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cargo-robocode-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn list_prints_built_ins() {
    let out = tool(&["list"], &temp_dir("list"));
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    for name in ["SafeShooter", "Circler", "Spinner", "SittingDuck"] {
        assert!(stdout.contains(name), "{stdout}");
    }
}

#[test]
fn works_as_cargo_subcommand() {
    // `cargo robocode list` runs `cargo-robocode robocode list`.
    let out = tool(&["robocode", "list"], &temp_dir("sub"));
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("SittingDuck"));
}

#[test]
fn new_writes_template() {
    let dir = temp_dir("new");
    let out = tool(&["new", "hunter"], &dir);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = std::fs::read_to_string(dir.join("hunter.rs")).unwrap();
    assert!(text.contains("robocode::robot!("), "{text}");
    assert!(text.contains("impl Robot for"), "{text}");
}

#[test]
fn new_refuses_overwrite() {
    let dir = temp_dir("overwrite");
    std::fs::write(dir.join("hunter.rs"), "// mine").unwrap();
    let out = tool(&["new", "hunter"], &dir);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("hunter.rs already exists"));
    assert_eq!(std::fs::read_to_string(dir.join("hunter.rs")).unwrap(), "// mine");
}

#[test]
fn unknown_command_exits_2() {
    let out = tool(&["fly"], &temp_dir("unknown"));
    assert_eq!(out.status.code(), Some(2));
}
