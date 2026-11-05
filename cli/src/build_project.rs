//! The generated Cargo projects robot files are compiled in.
//!
//! All projects share one target folder under the build directory, so the
//! robocode library compiles once and later battles are quick. Each distinct
//! battle program gets its own project (named by a hash of its source), so
//! two battles can run at the same time.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Failure;

/// Where generated projects and the shared target folder live.
pub fn build_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("ROBOCODE_BUILD_DIR") {
        return PathBuf::from(dir);
    }
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("robocode").join("build")
}

/// The robocode source to compile against: `--robocode-path`, then
/// `ROBOCODE_PATH`, then the checkout this tool was built from.
pub fn robocode_source(flag: Option<&Path>) -> Result<PathBuf, Failure> {
    let path = flag
        .map(Path::to_path_buf)
        .or_else(|| std::env::var_os("ROBOCODE_PATH").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(env!("ROBOCODE_SOURCE")));
    if path.join("Cargo.toml").is_file() && path.join("src").join("lib.rs").is_file() {
        Ok(path)
    } else {
        Err(Failure::usage(format!(
            "robocode source not found at {}; pass --robocode-path or set ROBOCODE_PATH",
            path.display()
        )))
    }
}

/// A generated project ready to build.
pub struct Project {
    pub dir: PathBuf,
    pub package: String,
    pub target_dir: PathBuf,
}

/// Writes (or reuses) a project whose `src/<main_file>` is `source`.
pub fn write_project(
    kind: &str,
    source: &str,
    main_file: &str,
    cargo_toml_extra: &str,
    robocode: &Path,
) -> Result<Project, Failure> {
    let mut hasher = DefaultHasher::new();
    (kind, source, main_file, cargo_toml_extra, robocode).hash(&mut hasher);
    let package = format!("{kind}_{:016x}", hasher.finish());
    let root = build_root();
    let dir = root.join("projects").join(&package);
    let io = |e: std::io::Error| Failure::usage(format!("could not write the build project in {}: {e}", dir.display()));
    std::fs::create_dir_all(dir.join("src")).map_err(io)?;
    let manifest = format!(
        "[package]\nname = \"{package}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n\
         [dependencies]\nrobocode = {{ path = {:?}{cargo_toml_extra} }}\n\n\
         # Standalone: never part of a surrounding workspace.\n[workspace]\n",
        robocode.display().to_string()
    );
    write_if_changed(&dir.join("Cargo.toml"), &manifest).map_err(io)?;
    write_if_changed(&dir.join("src").join(main_file), source).map_err(io)?;
    Ok(Project { dir, package, target_dir: root.join("target") })
}

fn write_if_changed(path: &Path, content: &str) -> std::io::Result<()> {
    if std::fs::read_to_string(path).ok().as_deref() != Some(content) {
        std::fs::write(path, content)?;
    }
    Ok(())
}

/// A `cargo` command in the project, sharing the target folder.
pub fn cargo(project: &Project, args: &[&str]) -> Command {
    let mut cmd = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    cmd.args(args)
        .arg("--manifest-path")
        .arg(project.dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &project.target_dir);
    cmd
}
