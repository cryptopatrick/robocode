use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// One build folder for every test in this file, so robocode compiles once.
fn build_dir() -> PathBuf {
    std::env::temp_dir().join("cargo-robocode-test-build")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cargo-robocode-battle-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tool(args: &[&str], dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-robocode"))
        .args(args)
        .current_dir(dir)
        .env("ROBOCODE_BUILD_DIR", build_dir())
        .env_remove("ROBOCODE_PATH")
        .output()
        .unwrap()
}

fn new_robot(dir: &Path, name: &str) {
    let out = tool(&["new", name], dir);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn battle_writes_recording() {
    let dir = temp_dir("writes");
    new_robot(&dir, "my_robot");
    let out = tool(&["battle", "my_robot.rs", "--vs", "SittingDuck", "--output", "battle.json", "--no-open"], &dir);
    assert!(out.status.success(), "{}", stderr(&out));
    let json = std::fs::read_to_string(dir.join("battle.json")).unwrap();
    assert!(json.starts_with(r#"{"format":"roboverus-recording","version":1,"robots":[{"id":0,"name":"MyRobot"},{"id":1,"name":"SittingDuck"}]"#), "{}", &json[..200.min(json.len())]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("MyRobot wins after"), "{}", String::from_utf8_lossy(&out.stdout));
}

#[test]
fn path_with_spaces_and_relative() {
    let dir = temp_dir("spaces");
    let sub = dir.join("my robots");
    std::fs::create_dir_all(&sub).unwrap();
    new_robot(&sub, "bot");
    let out = tool(&["battle", "my robots/bot.rs", "--output", "b.json", "--no-open"], &dir);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(std::fs::read_to_string(dir.join("b.json")).unwrap().contains(r#""name":"Bot""#));
}

#[test]
fn two_copies_of_same_file_and_names() {
    let dir = temp_dir("twice");
    new_robot(&dir, "bot");
    let out = tool(&["battle", "bot.rs", "--vs", "bot.rs", "--output", "b.json", "--no-open"], &dir);
    assert!(out.status.success(), "{}", stderr(&out));
    let json = std::fs::read_to_string(dir.join("b.json")).unwrap();
    assert!(json.contains(r#"{"id":0,"name":"Bot"},{"id":1,"name":"Bot (2)"}"#), "{}", &json[..200]);
}

#[test]
fn compile_error_exits_1_with_message() {
    let dir = temp_dir("broken");
    std::fs::write(dir.join("broken.rs"), "fn broken( {}\nrobocode::robot!(X);\n").unwrap();
    let out = tool(&["battle", "broken.rs", "--output", "b.json", "--no-open"], &dir);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains("error"), "{}", stderr(&out));
}

#[test]
fn missing_robot_macro_hint() {
    let dir = temp_dir("nomacro");
    new_robot(&dir, "bot");
    let text = std::fs::read_to_string(dir.join("bot.rs")).unwrap().replace("robocode::robot!(Bot);", "");
    std::fs::write(dir.join("bot.rs"), text).unwrap();
    let out = tool(&["battle", "bot.rs", "--output", "b.json", "--no-open"], &dir);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("add robocode::robot!(YourRobot); to"), "{}", stderr(&out));
}

#[test]
fn unknown_opponent_exits_2() {
    let dir = temp_dir("unknown");
    new_robot(&dir, "bot");
    let out = tool(&["battle", "bot.rs", "--vs", "Terminator", "--no-open"], &dir);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("unknown opponent: Terminator"), "{}", stderr(&out));
}

#[test]
fn too_many_robots_exits_2() {
    let dir = temp_dir("many");
    new_robot(&dir, "bot");
    let out = tool(&["battle", "bot.rs", "--vs", "SittingDuck", "--vs", "Spinner", "--vs", "Circler", "--vs", "SafeShooter", "--no-open"], &dir);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("need 2 to 4 robots, got 5"), "{}", stderr(&out));
}

#[test]
fn missing_source_exits_2() {
    let dir = temp_dir("nosource");
    new_robot(&dir, "bot");
    let out = tool(&["battle", "bot.rs", "--robocode-path", "/nonexistent/robocode", "--no-open"], &dir);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("robocode source not found at /nonexistent/robocode"), "{}", stderr(&out));
}

#[test]
fn second_run_reuses_cache() {
    let dir = temp_dir("cache");
    new_robot(&dir, "bot");
    let args = ["battle", "bot.rs", "--vs", "Spinner", "--output", "b.json", "--no-open", "--verbose"];
    let first = tool(&args, &dir);
    assert!(first.status.success(), "{}", stderr(&first));
    let second = tool(&args, &dir);
    assert!(second.status.success(), "{}", stderr(&second));
    assert!(!stderr(&second).contains("Compiling robocode"), "{}", stderr(&second));
}

#[test]
fn rounds_print_each_round_and_totals() {
    let dir = temp_dir("rounds");
    new_robot(&dir, "bot");
    let out = tool(&["battle", "bot.rs", "--rounds", "3", "--output", "b.json", "--no-open"], &dir);
    assert!(out.status.success(), "{}", stderr(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in ["Round 1:", "Round 2:", "Round 3:", "Wins: Bot ", "Score: Bot "] {
        assert!(stdout.contains(line), "missing {line:?} in:\n{stdout}");
    }
    assert!(std::fs::read_to_string(dir.join("b.json")).unwrap().starts_with(r#"{"format":"roboverus-recording""#));
}

#[test]
fn rounds_out_of_range_exits_2() {
    let dir = temp_dir("rounds-bad");
    new_robot(&dir, "bot");
    let out = tool(&["battle", "bot.rs", "--rounds", "0", "--no-open"], &dir);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("--rounds must be 1..=100"), "{}", stderr(&out));
}

#[test]
fn urban_battle_records_the_city_and_plain_battle_does_not() {
    let dir = temp_dir("urban");
    new_robot(&dir, "bot");
    let out = tool(&["battle", "bot.rs", "--urban", "--output", "urban.json", "--no-open"], &dir);
    assert!(out.status.success(), "{}", stderr(&out));
    let urban = std::fs::read_to_string(dir.join("urban.json")).unwrap();
    assert!(urban.contains(r#""terrain":{"buildings":[["#), "{}", &urban[..300.min(urban.len())]);

    let out = tool(&["battle", "bot.rs", "--output", "plain.json", "--no-open"], &dir);
    assert!(out.status.success(), "{}", stderr(&out));
    let plain = std::fs::read_to_string(dir.join("plain.json")).unwrap();
    assert!(!plain.contains("terrain"));
}
