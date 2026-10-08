use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::Path;

use robocode::Point;
use robocode::physics::{BattleConfig, BattleState, RobotState};

fn main() -> Result<(), Box<dyn Error>> {
    let args = CliArgs::parse(env::args().skip(1).collect())?;
    match args.mode {
        Mode::Generate { battle_id, turns, output_path } => {
            run_generation(&battle_id, turns, output_path.as_deref())?
        }
        Mode::Compare { expected, actual } => {
            if let Err(err) = compare_log_files(&expected, &actual) {
                eprintln!("Comparison failed: {err}");
                return Err(Box::new(err));
            }
            println!("Comparison succeeded: logs match.");
        }
        Mode::Manifest { path } => {
            run_manifest(&path)?;
        }
    }
    Ok(())
}

fn default_robots() -> Vec<RobotState> {
    let mut a = RobotState::new(0, Point::new(200.0, 500.0));
    a.gun_heading = 0.0;
    a.set_ahead(100.0);
    let mut b = RobotState::new(1, Point::new(600.0, 100.0));
    b.heading = 180.0;
    b.gun_heading = 180.0;
    b.set_ahead(100.0);
    vec![a, b]
}

fn run_generation(
    battle_id: &str,
    turns: u64,
    output_path: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let mut battle =
        BattleState::new(BattleConfig::default(), default_robots());
    let mut writer: Box<dyn Write> = match output_path {
        Some(path) => Box::new(BufWriter::new(create_output_file(path)?)),
        None => Box::new(BufWriter::new(io::stdout())),
    };

    for _ in 0..turns {
        let events = battle.step();
        let snapshot = battle.snapshot(battle_id, &events);
        snapshot.write_json(&mut writer)?;
        writer.write_all(b"\n")?;
    }

    writer.flush()?;
    Ok(())
}

fn create_output_file(path: &str) -> io::Result<File> {
    let path_obj = Path::new(path);
    if let Some(parent) = path_obj.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    File::create(path_obj)
}

fn run_manifest(path: &str) -> Result<(), Box<dyn Error>> {
    let entries = parse_manifest(path)?;
    if entries.is_empty() {
        println!("Manifest {path} did not contain any entries.");
        return Ok(());
    }
    println!("Running {} manifest entries...", entries.len());
    for entry in entries {
        let output_path =
            entry.output.unwrap_or_else(|| format!("{}.log", entry.battle_id));
        println!(
            "- Battle {}: {} turns → {}",
            entry.battle_id, entry.turns, output_path
        );
        run_generation(&entry.battle_id, entry.turns, Some(&output_path))?;
        if let Some(expected) = entry.expected.as_deref() {
            match compare_log_files(expected, &output_path) {
                Ok(_) => println!("  ✓ matches {}", expected),
                Err(err) => {
                    eprintln!("  ✗ mismatch vs {}: {err}", expected);
                    return Err(Box::new(err));
                }
            }
        }
    }
    Ok(())
}

struct CliArgs {
    mode: Mode,
}

enum Mode {
    Generate { battle_id: String, turns: u64, output_path: Option<String> },
    Compare { expected: String, actual: String },
    Manifest { path: String },
}

impl CliArgs {
    fn parse(tokens: Vec<String>) -> Result<Self, CliParseError> {
        let mut battle_id = String::from("rust-sim");
        let mut turns = 32;
        let mut output_path = None;
        let mut expected = None;
        let mut actual = None;
        let mut manifest = None;
        let mut iter = tokens.iter();
        while let Some(token) = iter.next() {
            match token.as_str() {
                "--battle-id" => {
                    if let Some(id) = iter.next() {
                        battle_id = id.clone();
                    }
                }
                "--turns" => {
                    if let Some(value) = iter.next() {
                        turns = value.parse::<u64>().map_err(|_| {
                            CliParseError(format!(
                                "Invalid value for --turns: {value}"
                            ))
                        })?;
                    }
                }
                "--output" => {
                    if let Some(path) = iter.next() {
                        output_path = Some(path.clone());
                    }
                }
                "--expected" => {
                    if let Some(path) = iter.next() {
                        expected = Some(path.clone());
                    }
                }
                "--actual" => {
                    if let Some(path) = iter.next() {
                        actual = Some(path.clone());
                    }
                }
                "--manifest" => {
                    if let Some(path) = iter.next() {
                        manifest = Some(path.clone());
                    }
                }
                "--help" | "-h" => {
                    return Err(CliParseError(usage()));
                }
                other => {
                    return Err(CliParseError(format!(
                        "Unknown argument: {other}\n{}",
                        usage()
                    )));
                }
            }
        }

        if let Some(path) = manifest {
            return Ok(Self { mode: Mode::Manifest { path } });
        }

        if expected.is_some() || actual.is_some() {
            let exp = expected.ok_or_else(|| {
                CliParseError(
                    "Missing --expected when using comparison mode"
                        .to_string(),
                )
            })?;
            let act = actual.ok_or_else(|| {
                CliParseError(
                    "Missing --actual when using comparison mode".to_string(),
                )
            })?;
            Ok(Self { mode: Mode::Compare { expected: exp, actual: act } })
        } else {
            Ok(Self { mode: Mode::Generate { battle_id, turns, output_path } })
        }
    }
}

fn usage() -> String {
    "Usage:\n  parity [--battle-id ID] [--turns N] [--output file]\n  parity --expected ref.log --actual rust.log\n".to_string()
}

#[derive(Debug)]
struct CliParseError(String);

impl std::fmt::Display for CliParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CliParseError {}

#[derive(Debug)]
enum CompareError {
    Io(io::Error),
    LengthMismatch {
        expected_lines: usize,
        actual_lines: usize,
    },
    Mismatch {
        line: usize,
        expected_turn: Option<u64>,
        actual_turn: Option<u64>,
        expected: Option<String>,
        actual: Option<String>,
    },
}

impl From<io::Error> for CompareError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl std::fmt::Display for CompareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompareError::Io(err) => write!(f, "I/O error: {err}"),
            CompareError::LengthMismatch { expected_lines, actual_lines } => {
                write!(
                    f,
                    "Log length mismatch (expected {}, actual {})",
                    expected_lines, actual_lines
                )
            }
            CompareError::Mismatch {
                line,
                expected_turn,
                actual_turn,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Mismatch on line {line} (expected turn {:?}, actual turn {:?})",
                    expected_turn, actual_turn
                )?;
                if let (Some(exp), Some(act)) = (expected, actual) {
                    write!(f, "\n  expected: {exp}\n    actual: {act}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for CompareError {}

#[derive(Debug)]
struct ManifestEntry {
    battle_id: String,
    turns: u64,
    output: Option<String>,
    expected: Option<String>,
}

#[derive(Debug)]
enum ManifestParseError {
    Io(io::Error),
    InvalidLine { line: usize, content: String },
    MissingField { line: usize, field: &'static str },
    InvalidValue { line: usize, message: String },
}

impl From<io::Error> for ManifestParseError {
    fn from(value: io::Error) -> Self {
        ManifestParseError::Io(value)
    }
}

impl std::fmt::Display for ManifestParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestParseError::Io(err) => write!(f, "I/O error: {err}"),
            ManifestParseError::InvalidLine { line, content } => {
                write!(f, "Invalid manifest line {line}: {content}")
            }
            ManifestParseError::MissingField { line, field } => {
                write!(f, "Missing required field `{field}` near line {line}")
            }
            ManifestParseError::InvalidValue { line, message } => {
                write!(f, "Invalid value on line {line}: {message}")
            }
        }
    }
}

impl std::error::Error for ManifestParseError {}

fn compare_log_files(
    expected_path: &str,
    actual_path: &str,
) -> Result<(), CompareError> {
    let expected_lines = read_lines(expected_path)?;
    let actual_lines = read_lines(actual_path)?;
    let max_len = expected_lines.len().max(actual_lines.len());

    for idx in 0..max_len {
        match (expected_lines.get(idx), actual_lines.get(idx)) {
            (Some(exp), Some(act)) => {
                if exp != act {
                    return Err(CompareError::Mismatch {
                        line: idx + 1,
                        expected_turn: extract_turn(exp),
                        actual_turn: extract_turn(act),
                        expected: Some(exp.clone()),
                        actual: Some(act.clone()),
                    });
                }
            }
            (Some(_), None) | (None, Some(_)) => {
                return Err(CompareError::LengthMismatch {
                    expected_lines: expected_lines.len(),
                    actual_lines: actual_lines.len(),
                });
            }
            (None, None) => break,
        }
    }

    Ok(())
}

fn read_lines(path: &str) -> Result<Vec<String>, io::Error> {
    let content = fs::read_to_string(Path::new(path))?;
    Ok(content.lines().map(|line| line.to_string()).collect())
}

fn extract_turn(line: &str) -> Option<u64> {
    let key = "\"turn\":";
    let idx = line.find(key)?;
    let mut rest = &line[idx + key.len()..];
    rest = rest.trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn parse_manifest(
    path: &str,
) -> Result<Vec<ManifestEntry>, ManifestParseError> {
    let content = fs::read_to_string(Path::new(path))?;
    let mut entries = Vec::new();
    let mut builder = ManifestEntryBuilder::default();
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let line_no = idx + 1;
        if trimmed.is_empty() {
            if let Some(entry) = builder.finish()? {
                entries.push(entry);
            }
            builder = ManifestEntryBuilder::default();
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        let (key, value) = trimmed.split_once('=').ok_or_else(|| {
            ManifestParseError::InvalidLine {
                line: line_no,
                content: line.to_string(),
            }
        })?;
        builder.apply(key.trim(), value.trim(), line_no)?;
    }
    if let Some(entry) = builder.finish()? {
        entries.push(entry);
    }
    Ok(entries)
}

#[derive(Default)]
struct ManifestEntryBuilder {
    battle_id: Option<String>,
    turns: Option<u64>,
    output: Option<String>,
    expected: Option<String>,
    seen_any: bool,
    last_line: usize,
}

impl ManifestEntryBuilder {
    fn apply(
        &mut self,
        key: &str,
        value: &str,
        line: usize,
    ) -> Result<(), ManifestParseError> {
        self.seen_any = true;
        self.last_line = line;
        match key {
            "battle_id" => {
                self.battle_id = Some(value.to_string());
            }
            "turns" => {
                let turns = value.parse::<u64>().map_err(|_| {
                    ManifestParseError::InvalidValue {
                        line,
                        message: format!(
                            "turns must be an integer, got `{value}`"
                        ),
                    }
                })?;
                self.turns = Some(turns);
            }
            "output" => {
                self.output = Some(value.to_string());
            }
            "expected" => {
                self.expected = Some(value.to_string());
            }
            other => {
                return Err(ManifestParseError::InvalidLine {
                    line,
                    content: format!("Unknown key `{other}`"),
                });
            }
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<Option<ManifestEntry>, ManifestParseError> {
        if !self.seen_any {
            return Ok(None);
        }
        let battle_id = self.battle_id.take().ok_or_else(|| {
            ManifestParseError::MissingField {
                line: self.last_line,
                field: "battle_id",
            }
        })?;
        let turns = self.turns.unwrap_or(32);
        let entry = ManifestEntry {
            battle_id,
            turns,
            output: self.output.take(),
            expected: self.expected.take(),
        };
        self.seen_any = false;
        Ok(Some(entry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn extract_turn_finds_value() {
        let turn = extract_turn("{\"turn\":42,\"robots\":[]}");
        assert_eq!(turn, Some(42));
    }

    #[test]
    fn compare_log_files_detects_mismatch() {
        let tmp_dir = std::env::temp_dir();
        let expected_path = tmp_dir.join("robocode_expected_log.jsonl");
        let actual_path = tmp_dir.join("robocode_actual_log.jsonl");
        write_file(&expected_path, "{\"turn\":1,\"a\":1}\n").unwrap();
        write_file(&actual_path, "{\"turn\":1,\"a\":2}\n").unwrap();

        let result = compare_log_files(
            expected_path.to_str().unwrap(),
            actual_path.to_str().unwrap(),
        );
        assert!(matches!(result, Err(CompareError::Mismatch { .. })));

        let _ = std::fs::remove_file(expected_path);
        let _ = std::fs::remove_file(actual_path);
    }

    #[test]
    fn compare_log_files_passes_for_equal_logs() {
        let tmp_dir = std::env::temp_dir();
        let expected_path = tmp_dir.join("robocode_expected_equal.jsonl");
        let actual_path = tmp_dir.join("robocode_actual_equal.jsonl");
        write_file(&expected_path, "{\"turn\":1}\n{\"turn\":2}\n").unwrap();
        write_file(&actual_path, "{\"turn\":1}\n{\"turn\":2}\n").unwrap();

        compare_log_files(
            expected_path.to_str().unwrap(),
            actual_path.to_str().unwrap(),
        )
        .unwrap();

        let _ = std::fs::remove_file(expected_path);
        let _ = std::fs::remove_file(actual_path);
    }

    fn write_file(path: &Path, contents: &str) -> io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(contents.as_bytes())
    }

    #[test]
    fn parse_manifest_multiple_entries() {
        let tmp_dir = std::env::temp_dir();
        let manifest_path = tmp_dir.join("robocode_manifest.parity");
        write_file(
            &manifest_path,
            "# Sample manifest\nbattle_id = demo\nturns = 10\noutput = logs/demo.log\nexpected = logs/demo.jvm\n\nbattle_id=second\nturns=5\n",
        )
        .unwrap();
        let entries = parse_manifest(manifest_path.to_str().unwrap()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].battle_id, "demo");
        assert_eq!(entries[0].turns, 10);
        assert_eq!(entries[0].output.as_deref(), Some("logs/demo.log"));
        assert_eq!(entries[0].expected.as_deref(), Some("logs/demo.jvm"));
        assert_eq!(entries[1].battle_id, "second");
        assert_eq!(entries[1].turns, 5);
        let _ = std::fs::remove_file(manifest_path);
    }

    #[test]
    fn parse_manifest_requires_battle_id() {
        let tmp_dir = std::env::temp_dir();
        let manifest_path = tmp_dir.join("robocode_manifest_invalid.parity");
        write_file(&manifest_path, "turns = 10").unwrap();
        let err = parse_manifest(manifest_path.to_str().unwrap()).unwrap_err();
        assert!(matches!(err, ManifestParseError::MissingField { .. }));
        let _ = std::fs::remove_file(manifest_path);
    }
}
