//! RoboCheck builds on robocode 0.5.0 without changing it: everything it adds
//! lives in robocheck/ (robocheck-arena is the extension).
use std::process::Command;

#[test]
fn robocode_core_is_v0_5_0() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
    let tag = Command::new("git").args(["rev-parse", "--verify", "--quiet", "v0.5.0"]).current_dir(root).output();
    match tag {
        Ok(out) if out.status.success() => {}
        _ => {
            eprintln!("skipped: git or the v0.5.0 tag is not available");
            return;
        }
    }
    let paths = ["src", "Cargo.toml", "exercises", "solutions", "wasm", "cli", "mutants", "web"];
    let diff = Command::new("git")
        .args(["diff", "--name-only", "v0.5.0", "--"])
        .args(paths)
        .current_dir(root)
        .output()
        .expect("git diff");
    let changed = String::from_utf8_lossy(&diff.stdout);
    assert!(changed.trim().is_empty(), "robocode files differ from v0.5.0:\n{changed}");
}
