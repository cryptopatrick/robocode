//! Embeds the battle viewer (web/dist) and records where the robocode
//! source lives, so generated robot projects can depend on it.
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn collect(dir: &Path, root: &Path, out: &mut Vec<(String, PathBuf)>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect(&path, root, out);
        } else {
            let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            out.push((rel, path));
        }
    }
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let source = manifest.parent().unwrap().canonicalize().unwrap();
    println!("cargo:rustc-env=ROBOCODE_SOURCE={}", source.display());

    let dist = source.join("web").join("dist");
    println!("cargo:rerun-if-changed={}", dist.display());
    if !dist.join("index.html").exists() {
        panic!("the battle viewer is not built: run `npm run wasm && npm run build` in web/ first");
    }
    let mut files = Vec::new();
    collect(&dist, &dist, &mut files);
    let mut code = String::from("pub static VIEWER_ASSETS: &[(&str, &[u8])] = &[\n");
    for (rel, path) in files {
        let _ = writeln!(code, "    ({rel:?}, include_bytes!({:?})),", path.display().to_string());
    }
    code.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("viewer_assets.rs");
    std::fs::write(out, code).unwrap();
}
