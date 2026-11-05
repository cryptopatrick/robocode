use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn get(port: u16, path: &str) -> (String, Vec<u8>) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(s, "GET {path} HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    (String::from_utf8_lossy(&raw[..split]).to_lowercase(), raw[split + 4..].to_vec())
}

#[test]
fn serves_viewer_and_recording() {
    let dir = std::env::temp_dir().join(format!("cargo-robocode-serve-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let exe = env!("CARGO_BIN_EXE_cargo-robocode");
    let build: PathBuf = std::env::temp_dir().join("cargo-robocode-test-build");
    assert!(Command::new(exe).args(["new", "bot"]).current_dir(&dir).status().unwrap().success());

    let mut child = Command::new(exe)
        .args(["battle", "bot.rs", "--no-open"])
        .current_dir(&dir)
        .env("ROBOCODE_BUILD_DIR", &build)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let url_line = lines
        .by_ref()
        .map(|l| l.unwrap())
        .find(|l| l.contains("http://127.0.0.1:"))
        .expect("the tool prints the address");
    let port: u16 = url_line.split("http://127.0.0.1:").nth(1).unwrap().split('/').next().unwrap().parse().unwrap();
    assert!(url_line.contains("/?recording=/battle.json"), "{url_line}");

    let (head, body) = get(port, "/battle.json");
    assert!(head.contains("200") && head.contains("application/json"), "{head}");
    assert!(String::from_utf8_lossy(&body).starts_with(r#"{"format":"roboverus-recording""#));

    let (head, body) = get(port, "/");
    assert!(head.contains("200") && head.contains("text/html"), "{head}");
    let html = String::from_utf8_lossy(&body).into_owned();
    assert!(html.contains(r#"<div id="root">"#));

    // The JS bundle named in index.html is served with a JavaScript type.
    let js = html.split('"').find(|s| s.starts_with("/assets/") && s.ends_with(".js")).expect("js asset");
    let (head, _) = get(port, js);
    assert!(head.contains("200") && head.contains("javascript"), "{head}");

    // The engine's .wasm file is named inside the JS bundle.
    let (_, js_body) = get(port, js);
    let js_text = String::from_utf8_lossy(&js_body).into_owned();
    let wasm_name = js_text
        .split(['"', '\'', '`', '/'])
        .find(|s| s.ends_with(".wasm"))
        .expect("wasm asset named in the bundle");
    let (head, body) = get(port, &format!("/assets/{wasm_name}"));
    assert!(head.contains("200") && head.contains("application/wasm"), "{head}");
    assert!(body.starts_with(b"\0asm"));

    let (head, _) = get(port, "/nope");
    assert!(head.contains("404"), "{head}");

    child.kill().unwrap();
    let _ = child.wait();
}
