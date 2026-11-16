//! The stored starter-rule runs (robocheck/stored/, made by scripts/robocheck_stored.sh):
//! one real run per subset of S1–S3, kept as it came out.

use std::path::{Path, PathBuf};

use serde_json::Value;

const COMBOS: [(&str, &[&str]); 8] = [
    ("s1-s2-s3", &["S1", "S2", "S3"]),
    ("s1-s2", &["S1", "S2"]),
    ("s1-s3", &["S1", "S3"]),
    ("s2-s3", &["S2", "S3"]),
    ("s1", &["S1"]),
    ("s2", &["S2"]),
    ("s3", &["S3"]),
    ("none", &[]),
];

fn stored() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("stored")
}

fn json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn stored_runs_cover_all_8_subsets_and_parse() {
    let manifest = json(&stored().join("manifest.json"));
    for (combo, ids) in COMBOS {
        assert_eq!(manifest["runs"][combo], format!("{combo}/run.json"), "{combo} missing from the manifest");
        let dir = stored().join(combo);
        let run = json(&dir.join("run.json"));
        assert_eq!(run["format"], "robocheck-run", "{combo}");
        assert_eq!(run["source"], "stored", "{combo}");
        let got: Vec<&str> = run["rules"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap()).collect();
        assert_eq!(got, ids, "{combo}");
        // Every file run.json names is there (recordings gzipped).
        for arena in ["open", "urban"] {
            if let Some(rec) = run["battles"][arena]["recording"].as_str() {
                assert!(dir.join(format!("{rec}.gz")).exists(), "{combo}: {rec}.gz");
            }
        }
        assert!(dir.join("evidence.json").exists(), "{combo}");
    }
}

#[test]
fn stored_runs_record_model_and_version() {
    let manifest = json(&stored().join("manifest.json"));
    for key in ["created_at", "model", "robocode", "verus"] {
        assert!(manifest[key].as_str().is_some_and(|s| !s.is_empty()), "manifest.{key}");
    }
    for (combo, _) in COMBOS {
        let run = json(&stored().join(combo).join("run.json"));
        assert_eq!(run["model"], manifest["model"], "{combo}");
        assert_eq!(run["robocode"], manifest["robocode"], "{combo}");
    }
}
