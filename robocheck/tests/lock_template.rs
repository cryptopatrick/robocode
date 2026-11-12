//! The candidate build runs with `--locked`, so the lock template must pin the
//! same `robocode` version as the crate. Bumping the version without the
//! template breaks every candidate build; this test catches that.

#[test]
fn lock_template_pins_the_current_robocode_version() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.toml")).unwrap();
    let version = manifest
        .lines()
        .find_map(|l| l.strip_prefix("version = \"").and_then(|v| v.strip_suffix('"')))
        .expect("the root Cargo.toml has a version");
    let template = include_str!("../template/Cargo.lock.template");
    let pinned = format!("name = \"robocode\"\nversion = \"{version}\"");
    assert!(template.contains(&pinned), "robocheck/template/Cargo.lock.template must pin robocode {version}");
}
