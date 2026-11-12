//! `cargo robocode battle --live`: the battle runs while it is streamed.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn build_dir() -> PathBuf {
    std::env::temp_dir().join("cargo-robocode-test-build")
}

fn exe() -> &'static str {
    env!("CARGO_BIN_EXE_cargo-robocode")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cargo-robocode-live-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    assert!(Command::new(exe()).args(["new", "bot"]).current_dir(&dir).status().unwrap().success());
    dir
}

/// Starts a live battle and returns the child and the server port.
fn start(dir: &PathBuf, tps: &str) -> (Child, u16) {
    let mut child = Command::new(exe())
        .args(["battle", "bot.rs", "--live", "--tps", tps, "--no-open"])
        .current_dir(dir)
        .env("ROBOCODE_BUILD_DIR", build_dir())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let line = lines.by_ref().map(|l| l.unwrap()).find(|l| l.contains("http://127.0.0.1:")).expect("address");
    assert!(line.contains("/?live=/live"), "{line}");
    let port = line.split("http://127.0.0.1:").nth(1).unwrap().split('/').next().unwrap().parse().unwrap();
    (child, port)
}

fn request(port: u16, method: &str, path: &str, body: &str) -> (String, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(s, "{method} {path} HTTP/1.0\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    let mut raw = String::new();
    s.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
    (head.to_lowercase(), body.to_string())
}

/// Reads SSE `data:` lines from /live in a background thread; returns a receiver
/// of (arrival time, json).
fn subscribe(port: u16) -> std::sync::mpsc::Receiver<(Instant, String)> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(s, "GET /live HTTP/1.1\r\nHost: x\r\nAccept: text/event-stream\r\n\r\n").unwrap();
    std::thread::spawn(move || {
        for line in BufReader::new(s).lines() {
            let Ok(line) = line else { break };
            if let Some(data) = line.strip_prefix("data: ")
                && tx.send((Instant::now(), data.to_string())).is_err()
            {
                break;
            }
        }
    });
    rx
}

fn turn_of(frame: &str) -> u64 {
    let i = frame.find(r#""turn":{"battle_id":"viewer","turn":"#).expect("a frame");
    let rest = &frame[i + r#""turn":{"battle_id":"viewer","turn":"#.len()..];
    rest[..rest.find(',').unwrap()].parse().unwrap()
}

#[test]
fn frames_arrive_while_the_battle_runs_and_pause_stops_them() {
    let dir = temp_dir("pace");
    let (mut child, port) = start(&dir, "30");
    let rx = subscribe(port);
    let (_, header) = rx.recv_timeout(Duration::from_secs(60)).unwrap();
    assert!(header.starts_with(r#"{"format":"roboverus-recording","version":1"#), "{header}");

    // Not precomputed: frames arrive while the recording doesn't exist yet.
    let (_, first) = rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let (head, _) = request(port, "GET", "/battle.json", "");
    assert!(head.contains("409"), "battle still running: {head}");

    // About 30 turns per second.
    let start_turn = turn_of(&first);
    std::thread::sleep(Duration::from_millis(1000));
    let mut last = start_turn;
    while let Ok((_, f)) = rx.try_recv() {
        last = turn_of(&f);
    }
    let rate = last - start_turn;
    assert!((20..=40).contains(&rate), "{rate} turns in 1 s at 30 TPS");

    // Pause: no new turns.
    let (head, _) = request(port, "POST", "/control", "pause");
    assert!(head.contains("200"), "{head}");
    std::thread::sleep(Duration::from_millis(200));
    while rx.try_recv().is_ok() {}
    std::thread::sleep(Duration::from_millis(600));
    assert!(rx.try_recv().is_err(), "frames kept coming while paused");

    // Step while paused: exactly one more turn.
    request(port, "POST", "/control", "step");
    let (_, stepped) = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(rx.try_recv().is_err(), "a step is one turn");
    assert!(turn_of(&stepped) > last);

    child.kill().unwrap();
    let _ = child.wait();
}

#[test]
fn finished_live_battle_records_the_same_battle() {
    let dir = temp_dir("same");
    let plain = Command::new(exe())
        .args(["battle", "bot.rs", "--output", "plain.json", "--no-open"])
        .current_dir(&dir)
        .env("ROBOCODE_BUILD_DIR", build_dir())
        .output()
        .unwrap();
    assert!(plain.status.success(), "{}", String::from_utf8_lossy(&plain.stderr));

    let (mut child, port) = start(&dir, "max");
    let rx = subscribe(port);
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let (_, line) = rx.recv_timeout(deadline - Instant::now()).expect("the stream ends with a done line");
        if line.contains(r#""done":true"#) {
            break;
        }
    }
    let (head, body) = request(port, "GET", "/battle.json", "");
    assert!(head.contains("200"), "{head}");
    assert_eq!(body, std::fs::read_to_string(dir.join("plain.json")).unwrap());
    child.kill().unwrap();
    let _ = child.wait();
}

#[test]
fn live_with_several_rounds_is_a_usage_error() {
    let dir = temp_dir("rounds");
    let out = Command::new(exe())
        .args(["battle", "bot.rs", "--live", "--rounds", "3", "--no-open"])
        .current_dir(&dir)
        .env("ROBOCODE_BUILD_DIR", build_dir())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--live plays one round"), "{}", String::from_utf8_lossy(&out.stderr));
}
