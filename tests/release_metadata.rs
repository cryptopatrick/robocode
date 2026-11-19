//! Release metadata must agree with the crate version. These are the files
//! that went out of step by hand before: the README's install line and the
//! changelog's section for the current version.

use std::path::Path;

fn read(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn version() -> String {
    read("Cargo.toml")
        .lines()
        .find_map(|l| {
            l.strip_prefix("version = \"").and_then(|v| v.strip_suffix('"'))
        })
        .expect("Cargo.toml has a version")
        .to_string()
}

#[test]
fn the_readme_install_line_matches_the_version() {
    let v = version();
    let minor = v.rsplit_once('.').map(|(m, _)| m).expect("major.minor.patch");
    let install = format!("robocode = \"{minor}\"");
    assert!(
        read("README.md").contains(&install),
        "README.md should say `{install}` for version {v}"
    );
}

#[test]
fn the_changelog_has_a_section_for_the_version() {
    let v = version();
    let heading = format!("## v{v} - ");
    assert!(
        read("CHANGELOG.md").lines().any(|l| l.starts_with(&heading)),
        "CHANGELOG.md has no section `{heading}...` for the current version"
    );
}
