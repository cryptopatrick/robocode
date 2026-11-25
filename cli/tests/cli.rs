use std::path::PathBuf;
use std::process::{Command, Output};

fn tool(args: &[&str], dir: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-robocode"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("cargo-robocode-test-{name}-{}", std::process::id()));
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
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("hunter.rs already exists")
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("hunter.rs")).unwrap(),
        "// mine"
    );
}

#[test]
fn unknown_command_exits_2() {
    let out = tool(&["fly"], &temp_dir("unknown"));
    assert_eq!(out.status.code(), Some(2));
}

fn open_arena_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/open_arena_0_5_0.json")
}

#[test]
fn export_writes_glb() {
    let dir = temp_dir("export");
    let fixture = open_arena_fixture();
    let out =
        tool(&["export", fixture.to_str().unwrap(), "--glb", "a.glb"], &dir);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{err}");
    assert!(err.contains("Saved the 3D scene to a.glb"), "{err}");
    let bytes = std::fs::read(dir.join("a.glb")).unwrap();
    assert_eq!(&bytes[..4], b"glTF");
}

#[test]
fn export_rejects_non_recording() {
    let dir = temp_dir("export-bad");
    std::fs::write(dir.join("x.json"), "{}").unwrap();
    let out = tool(&["export", "x.json", "--glb", "a.glb"], &dir);
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("x.json is not a robocode recording"), "{err}");
    assert_eq!(err.matches("not a robocode recording").count(), 1, "{err}");
    assert!(!dir.join("a.glb").exists());
}

#[test]
fn export_reports_missing_file() {
    let dir = temp_dir("export-missing");
    let out = tool(&["export", "nope.json", "--glb", "a.glb"], &dir);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("could not read nope.json")
    );
}

#[test]
fn export_reports_unwritable_output() {
    let dir = temp_dir("export-unwritable");
    let fixture = open_arena_fixture();
    let out = tool(
        &["export", fixture.to_str().unwrap(), "--glb", "no/such/dir/a.glb"],
        &dir,
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("could not write no/such/dir/a.glb")
    );
}

#[test]
fn glb_conflicts_with_live() {
    let dir = temp_dir("glb-live");
    let out = tool(&["battle", "x.rs", "--live", "--glb", "a.glb"], &dir);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("cannot be used with")
    );
}
