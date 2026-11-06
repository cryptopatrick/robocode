//! Writes a standalone candidate crate into a work directory.

use std::path::{Path, PathBuf};

use crate::template::{LIB_RS, sha256_hex};

const MANIFEST_IN: &str = include_str!("../template/Cargo.toml.in");
const LOCKFILE: &str = include_str!("../template/Cargo.lock.template");
const BATTLE_RS: &str = include_str!("../template/battle.rs");

/// The robocode repository this tool belongs to.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("robocheck lives inside the robocode repo").to_path_buf()
}

/// Target dir for plain builds of candidate crates, shared across runs.
pub fn plain_target_dir() -> PathBuf {
    repo_root().join("target/robocheck/target-plain")
}

/// Target dir for Verus builds; separate from plain builds, which must not
/// share a target dir with `cargo verus`.
pub fn verus_target_dir() -> PathBuf {
    repo_root().join("target/robocheck/target-verus")
}

/// Name of the candidate crate and of its battle binary for this `strategy.rs`.
/// Unique per implementation: candidate crates share a target dir, and Cargo
/// fingerprints a root package by a workspace-relative path, so two
/// candidates with one name could be served each other's build.
pub fn crate_and_bin(strategy_rs: &str) -> (String, String) {
    let h = &sha256_hex(strategy_rs.as_bytes())[..12];
    (format!("robocheck-candidate-{h}"), format!("battle-{h}"))
}

/// Writes a standalone candidate crate (its own `[workspace]`) into `dir`.
/// Returns the name of its battle binary.
pub fn materialize(dir: &Path, strategy_rs: &str, repo_root: &Path) -> std::io::Result<String> {
    std::fs::create_dir_all(dir.join("src/bin"))?;
    let (krate, bin) = crate_and_bin(strategy_rs);
    let path = repo_root.display().to_string();
    let manifest = MANIFEST_IN.replace("{{ROBOCODE_PATH}}", &path).replace("{{CRATE}}", &krate).replace("{{BIN}}", &bin);
    std::fs::write(dir.join("Cargo.toml"), manifest)?;
    let lock = LOCKFILE.replace("name = \"robocheck-candidate\"", &format!("name = \"{krate}\""));
    std::fs::write(dir.join("Cargo.lock"), lock)?;
    std::fs::write(dir.join("src/lib.rs"), LIB_RS)?;
    std::fs::write(dir.join("src/strategy.rs"), strategy_rs)?;
    std::fs::write(dir.join("src/bin/battle.rs"), BATTLE_RS.replace("{{LIB}}", &krate.replace('-', "_")))?;
    Ok(bin)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A fresh, empty work directory for a test.
    pub(crate) fn temp_dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("robocheck-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn manifest_points_at_this_repo() {
        let dir = temp_dir("manifest");
        materialize(&dir, "// strategy", &repo_root()).unwrap();
        let m = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap();
        assert!(m.contains(&format!("path = \"{}\"", repo_root().display())));
        assert!(m.contains("[workspace]"));
        for f in ["Cargo.lock", "src/lib.rs", "src/strategy.rs", "src/bin/battle.rs"] {
            assert!(dir.join(f).exists(), "{f}");
        }
        assert_eq!(std::fs::read_to_string(dir.join("src/strategy.rs")).unwrap(), "// strategy");
    }
}
