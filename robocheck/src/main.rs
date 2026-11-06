//! `robocheck run <candidate.rs> [--opponent NAME] [--turns N] [--out DIR] [--timeout SECS]`
//!
//! Exit code 0 when the pipeline completed (whatever the verdicts), 2 when the
//! tool itself could not run.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use robocheck::pipeline::{Options, run};
use robocheck::verify::CargoVerus;

const USAGE: &str = "usage: robocheck run <candidate.rs> [--opponent NAME] [--turns N] [--out DIR] [--timeout SECS]";

fn value(args: &[String], key: &str) -> Option<String> {
    args.iter().position(|a| a == key).and_then(|i| args.get(i + 1)).cloned()
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some("run"), Some(candidate)) = (args.first().map(String::as_str), args.get(1)) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let parse_num = |key: &str, default: u64| value(&args, key).map_or(Ok(default), |v| v.parse::<u64>());
    let (Ok(turns), Ok(timeout)) = (parse_num("--turns", 2000), parse_num("--timeout", 300)) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let options = Options {
        candidate: PathBuf::from(candidate),
        opponent: value(&args, "--opponent").unwrap_or_else(|| "SittingDuck".into()),
        turns,
        out: value(&args, "--out").map(PathBuf::from),
        verus_dir: CargoVerus::default_verus_dir(),
        timeout: Duration::from_secs(timeout),
    };
    match run(&options) {
        Ok((out, ev)) => {
            if !ev.validation.ok {
                println!("refused before verification:");
                ev.validation.problems.iter().for_each(|p| println!("  - {p}"));
            }
            println!("build: {}", ev.build.status);
            for p in &ev.verification.properties {
                println!("{:<9} {:<13} {}", p.id, format!("{:?}", p.status).to_lowercase(), p.name);
            }
            match ev.battle.status {
                "ok" => println!(
                    "battle vs {}: {} after {} turns; same implementation as verified: {}",
                    ev.battle.opponent,
                    ev.battle.winner.as_deref().map_or("no winner".to_string(), |w| format!("{w} wins")),
                    ev.battle.turns.unwrap_or(0),
                    ev.battle.same_artifact.unwrap_or(false)
                ),
                s => println!("battle: {s}"),
            }
            println!("evidence: {}", out.join("evidence.json").display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}
