//! Mutation checks for robocode's Verus specs.
//!
//! A proof only means something if its spec constrains the code. For every
//! `ensures` clause we plant one bug (a *mutant*) that the clause should
//! catch, and require Verus to reject it, naming that clause. A mutant that
//! verifies is a spec that constrains nothing (Proof-Driven Development's
//! "mutation check").
//!
//! Usage: `cargo run -p verus-mutants [-- <id substring>]` (needs `cargo verus`).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// One planted bug.
#[derive(Debug, Clone, PartialEq)]
pub struct Mutant {
    pub id: String,
    pub file: String,
    pub find: String,
    pub replace: String,
    /// Text from the targeted clause that must appear in Verus's errors.
    pub expect: String,
}

/// Parses the manifest: blocks of `key = value` lines separated by blank
/// lines; `#` starts a comment; `\n` in a value means a newline.
pub fn parse_manifest(text: &str) -> Result<Vec<Mutant>, String> {
    let mut mutants = Vec::new();
    for (n, block) in text.split("\n\n").enumerate() {
        let mut fields = std::collections::HashMap::new();
        for line in block.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
            let (k, v) = line.split_once(" = ").ok_or_else(|| format!("block {}: bad line `{line}`", n + 1))?;
            fields.insert(k.trim().to_string(), v.replace("\\n", "\n"));
        }
        if fields.is_empty() {
            continue;
        }
        let get = |k: &str| fields.get(k).cloned().ok_or_else(|| format!("block {}: missing `{k}`", n + 1));
        mutants.push(Mutant { id: get("id")?, file: get("file")?, find: get("find")?, replace: get("replace")?, expect: get("expect")? });
    }
    Ok(mutants)
}

/// Applies a mutant to `source`; the `find` text must occur exactly once.
pub fn apply(source: &str, m: &Mutant) -> Result<String, String> {
    match source.matches(&m.find).count() {
        1 => Ok(source.replacen(&m.find, &m.replace, 1)),
        0 => Err(format!("{}: `find` text not found in {}", m.id, m.file)),
        k => Err(format!("{}: `find` text occurs {k} times in {}", m.id, m.file)),
    }
}

/// Copies the crate's manifest, lockfile and sources into `dest`, as a
/// standalone package (no workspace).
fn prepare_copy(repo: &Path, dest: &Path) -> std::io::Result<()> {
    let _ = std::fs::remove_dir_all(dest.join("src"));
    std::fs::create_dir_all(dest)?;
    let manifest = std::fs::read_to_string(repo.join("Cargo.toml"))?;
    let manifest = match manifest.find("\n[workspace]") {
        Some(i) => format!("{}\n[workspace]\n", &manifest[..i]),
        None => format!("{manifest}\n[workspace]\n"),
    };
    std::fs::write(dest.join("Cargo.toml"), manifest)?;
    if repo.join("Cargo.lock").exists() {
        std::fs::copy(repo.join("Cargo.lock"), dest.join("Cargo.lock"))?;
    }
    copy_dir(&repo.join("src"), &dest.join("src"))
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn verify(dir: &Path) -> (bool, String) {
    let out = Command::new("cargo")
        .args(["verus", "verify", "--features", "verus"])
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", dir.join("target"))
        .output()
        .expect("run cargo verus");
    (out.status.success(), format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)))
}

fn main() -> ExitCode {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let filter = std::env::args().nth(1);
    let manifest = std::fs::read_to_string(repo.join("mutants").join("mutants.txt")).expect("read mutants/mutants.txt");
    let mutants = match parse_manifest(&manifest) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let work = std::env::temp_dir().join("robocode-verus-mutants");
    if let Err(e) = prepare_copy(&repo, &work) {
        eprintln!("error: could not copy the crate: {e}");
        return ExitCode::from(2);
    }
    let (ok, log) = verify(&work);
    if !ok {
        eprintln!("{log}\nerror: the unmutated crate does not verify");
        return ExitCode::from(2);
    }
    println!("baseline verifies; checking mutants\n");
    println!("{:<34} {:<9} targeted clause", "mutant", "result");
    let mut survivors = 0;
    let mut broken = 0;
    for m in mutants.iter().filter(|m| filter.as_deref().is_none_or(|f| m.id.contains(f))) {
        let path = work.join(&m.file);
        let original = std::fs::read_to_string(&path).unwrap_or_default();
        let mutated = match apply(&original, m) {
            Ok(s) => s,
            Err(e) => {
                println!("{:<34} {:<9} {e}", m.id, "BROKEN");
                broken += 1;
                continue;
            }
        };
        std::fs::write(&path, mutated).unwrap();
        let (ok, log) = verify(&work);
        std::fs::write(&path, original).unwrap();
        let (result, note) = if ok {
            survivors += 1;
            ("SURVIVED", format!("{}  <- the clause constrains nothing here", m.expect))
        } else if log.contains(&m.expect) {
            ("killed", m.expect.clone())
        } else {
            survivors += 1;
            ("MISSED", format!("{}  <- Verus failed, but not on this clause", m.expect))
        };
        println!("{:<34} {:<9} {note}", m.id, result);
    }
    println!();
    if survivors + broken > 0 {
        println!("{survivors} mutant(s) survived, {broken} could not be applied");
        ExitCode::FAILURE
    } else {
        println!("every mutant was killed by its clause");
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_blocks_and_newlines() {
        let m = parse_manifest("# c\nid = a\nfile = f.rs\nfind = x\\ny\nreplace = z\nexpect = e\n\nid = b\nfile = g.rs\nfind = 1\nreplace = 2\nexpect = q\n").unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].find, "x\ny");
        assert_eq!(m[1].id, "b");
    }

    #[test]
    fn missing_field_is_an_error() {
        assert!(parse_manifest("id = a\nfile = f\n").unwrap_err().contains("missing `find`"));
    }

    #[test]
    fn apply_requires_exactly_one_match() {
        let m = Mutant { id: "m".into(), file: "f".into(), find: "ab".into(), replace: "X".into(), expect: "e".into() };
        assert_eq!(apply("--ab--", &m).unwrap(), "--X--");
        assert!(apply("none", &m).unwrap_err().contains("not found"));
        assert!(apply("ab ab", &m).unwrap_err().contains("2 times"));
    }
}
