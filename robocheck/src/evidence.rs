//! The evidence file: everything one run produced, tied to one implementation hash.

use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::verify::PropertyResult;

#[derive(Serialize, Debug, Clone)]
pub struct Evidence {
    pub format: &'static str,
    pub version: u32,
    pub run_id: String,
    pub created_at: String,
    pub candidate: CandidateInfo,
    /// SHA-256 of the assembled `strategy.rs`: the implementation that was verified and battled.
    pub artifact_hash: Option<String>,
    /// SHA-256 of the trusted header holding the reference spec.
    pub spec_hash: String,
    pub validation: Validation,
    pub build: Build,
    pub verification: Verification,
    pub battle: BattleInfo,
    pub toolchain: Toolchain,
}

#[derive(Serialize, Debug, Clone)]
pub struct CandidateInfo {
    pub path: String,
    pub sha256: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct Validation {
    pub ok: bool,
    pub problems: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Build {
    /// `ok`, `failed` or `skipped`.
    pub status: &'static str,
    pub log_file: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Verification {
    pub properties: Vec<PropertyResult>,
    pub log_files: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct BattleInfo {
    /// `ok`, `failed`, `timed_out` or `skipped`.
    pub status: &'static str,
    pub opponent: String,
    pub turns: Option<u64>,
    pub winner: Option<String>,
    pub artifact_hash: Option<String>,
    pub same_artifact: Option<bool>,
    pub recording_file: Option<String>,
    pub recording_sha256: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Toolchain {
    pub robocode: String,
    pub verus: String,
    pub rustc: String,
    pub z3: String,
}

fn first_line(cmd: &mut Command) -> String {
    cmd.output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8_lossy(&o.stdout).lines().find(|l| !l.trim().is_empty()).map(str::to_string))
        .unwrap_or_else(|| "unavailable".into())
}

/// The version from `verus --version` output ("Verus\n  Version: X\n...").
pub fn verus_version(output: &str) -> String {
    output
        .lines()
        .find_map(|l| l.trim().strip_prefix("Version:"))
        .map_or_else(|| "unavailable".to_string(), |v| v.trim().to_string())
}

/// Versions of the tools a verdict depends on.
pub fn toolchain(repo_root: &Path, verus_dir: &Path) -> Toolchain {
    let manifest = std::fs::read_to_string(repo_root.join("Cargo.toml")).unwrap_or_default();
    let robocode = manifest
        .lines()
        .find_map(|l| l.strip_prefix("version = \"").and_then(|v| v.strip_suffix('"')))
        .unwrap_or("unknown")
        .to_string();
    Toolchain {
        robocode,
        verus: Command::new(verus_dir.join("verus"))
            .arg("--version")
            .output()
            .map_or_else(|_| "unavailable".to_string(), |o| verus_version(&String::from_utf8_lossy(&o.stdout))),
        rustc: first_line(Command::new("rustc").args(["+1.99.1", "--version"])),
        z3: first_line(Command::new(verus_dir.join("z3")).arg("--version")),
    }
}

/// UTC timestamp in RFC 3339, from the Unix time in seconds.
pub fn rfc3339(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Civil-from-days (H. Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verus_version_is_read_from_its_version_line() {
        let out = "Verus\n  Version: 0.2026.11.02.8bf68ee\n  Profile: release\n";
        assert_eq!(verus_version(out), "0.2026.11.02.8bf68ee");
        assert_eq!(verus_version("garbage"), "unavailable");
    }

    #[test]
    fn rfc3339_known_dates() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_793_966_400), "2026-11-06T12:00:00Z");
    }
}
