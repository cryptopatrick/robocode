//! `robocheck run <candidate.rs> [--opponent NAME] [--turns N] [--out DIR] [--timeout SECS] [--battle-timeout SECS]`
//! `robocheck rules <rules.json> [--opponent NAME] [--turns N] [--out DIR] [--timeout SECS] [--battle-timeout SECS] [--stored]`
//!
//! Exit code 0 when the pipeline completed (whatever the verdicts), 2 when the
//! tool itself could not run.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use robocheck::pipeline::{Options, run};
use robocheck::verify::CargoVerus;

const BATTLE_USAGE: &str = "usage: robocheck battle <strategy.rs> --out DIR [--opponents A,B,C] [--tanks T,T,T,T] [--width PX --height PX] [--arena open|urban | --buildings x,y,w,h;...] [--bullets realistic|arcade] [--turns N] [--battle-timeout SECS]";

/// `robocheck battle`: fights an already-assembled strategy.rs in a chosen arena, no AI and no Verus.
/// Writes DIR/recording.json (the battle program's own output) and prints its summary line.
fn battle(args: &[String], strategy: &str) -> ExitCode {
    use robocheck::compile::{BuildOutcome, build};
    use robocheck::workspace::{
        crate_and_bin, materialize, plain_target_dir, repo_root,
    };
    let Some(out) = value(args, "--out").map(PathBuf::from) else {
        eprintln!("{BATTLE_USAGE}");
        return ExitCode::from(2);
    };
    let timeout = value(args, "--battle-timeout")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(120);
    let run = || -> Result<String, String> {
        let source = std::fs::read_to_string(strategy)
            .map_err(|e| format!("{strategy}: {e}"))?;
        std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        let work = out.join("crate");
        materialize(&work, &source, &repo_root())
            .map_err(|e| e.to_string())?;
        if let BuildOutcome::Failed { log } = build(&work, &plain_target_dir())
        {
            return Err(format!("battle build failed:\n{log}"));
        }
        let (_, bin) = crate_and_bin(&source);
        let exe = plain_target_dir().join("release").join(bin);
        // Everything after the strategy path that the battle program understands is passed through.
        let mut cmd = std::process::Command::new(&exe);
        for key in [
            "--opponents",
            "--tanks",
            "--width",
            "--height",
            "--arena",
            "--buildings",
            "--bullets",
            "--turns",
        ] {
            if let Some(v) = value(args, key) {
                cmd.arg(key).arg(v);
            }
        }
        cmd.arg("--out")
            .arg(out.join("recording.json"))
            .arg("--decisions")
            .arg(out.join("decisions.jsonl"));
        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("cannot run {}: {e}", exe.display()))?;
        let start = std::time::Instant::now();
        loop {
            if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                break;
            }
            if start.elapsed() > Duration::from_secs(timeout) {
                let _ = child.kill();
                return Err(format!(
                    "the battle exceeded its time limit of {timeout} s"
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let o = child.wait_with_output().map_err(|e| e.to_string())?;
        if !o.status.success() {
            return Err(String::from_utf8_lossy(&o.stderr).trim().to_string());
        }
        Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    match run() {
        Ok(summary) => {
            println!("{summary}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

const USAGE: &str = "usage: robocheck run <candidate.rs> [--opponent NAME] [--turns N] [--out DIR] [--timeout SECS] [--battle-timeout SECS]
       robocheck rules <rules.json> [--opponent NAME] [--turns N] [--out DIR] [--timeout SECS] [--battle-timeout SECS] [--stored]";

/// `robocheck rules`: needs ANTHROPIC_API_KEY and Verus.
fn rules(args: &[String], file: &str) -> ExitCode {
    use robocheck::llm::ClaudeClient;
    use robocheck::rules_pipeline::{RulesOptions, run_rules};
    use robocheck::verify::Verifier;
    let parse_num = |key: &str, default: u64| {
        value(args, key).map_or(Ok(default), |v| v.parse::<u64>())
    };
    let (Ok(turns), Ok(timeout), Ok(battle_timeout)) = (
        parse_num("--turns", 2000),
        parse_num("--timeout", 300),
        parse_num("--battle-timeout", 120),
    ) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let mut llm = match ClaudeClient::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let options = RulesOptions {
        rules_file: PathBuf::from(file),
        opponent: value(args, "--opponent")
            .unwrap_or_else(|| "Circler".into()),
        turns,
        out: value(args, "--out").map(PathBuf::from),
        battle_timeout: Duration::from_secs(battle_timeout),
        source: if args.iter().any(|a| a == "--stored") {
            "stored".into()
        } else {
            "live".into()
        },
    };
    let mut make = |dir: &std::path::Path| -> Box<dyn Verifier> {
        Box::new(CargoVerus {
            dir: dir.to_path_buf(),
            target_dir: robocheck::workspace::verus_target_dir(),
            verus_dir: CargoVerus::default_verus_dir(),
            timeout: Duration::from_secs(timeout),
        })
    };
    match run_rules(&options, &mut llm, &mut make) {
        Ok((out, page)) => {
            println!(
                "outcome: {}{}",
                page.outcome,
                page.outcome_note
                    .as_deref()
                    .map(|n| format!(" ({n})"))
                    .unwrap_or_default()
            );
            for r in &page.rules {
                println!("{:<3} {:<15} {}", r.id, r.status, r.text);
                if let Some(rb) = &r.read_back {
                    println!("    understood as: {rb}");
                }
            }
            for d in &page.deleted_starters {
                println!("{:<3} {:<15} {}", d.id, "deleted", d.text);
            }
            if let Some(e) = &page.explanation {
                println!("{}", e.headline);
            }
            println!(
                "cost: ${:.4}; same implementation verified and battled: {}",
                page.cost_usd, page.same_artifact
            );
            println!("run: {}", out.join("run.json").display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

fn value(args: &[String], key: &str) -> Option<String> {
    args.iter().position(|a| a == key).and_then(|i| args.get(i + 1)).cloned()
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let (Some("battle"), Some(file)) =
        (args.first().map(String::as_str), args.get(1))
    {
        return battle(&args, file);
    }
    if let (Some("rules"), Some(file)) =
        (args.first().map(String::as_str), args.get(1))
    {
        return rules(&args, file);
    }
    let (Some("run"), Some(candidate)) =
        (args.first().map(String::as_str), args.get(1))
    else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let parse_num = |key: &str, default: u64| {
        value(&args, key).map_or(Ok(default), |v| v.parse::<u64>())
    };
    let (Ok(turns), Ok(timeout), Ok(battle_timeout)) = (
        parse_num("--turns", 2000),
        parse_num("--timeout", 300),
        parse_num("--battle-timeout", 120),
    ) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let options = Options {
        candidate: PathBuf::from(candidate),
        opponent: value(&args, "--opponent")
            .unwrap_or_else(|| "SittingDuck".into()),
        turns,
        out: value(&args, "--out").map(PathBuf::from),
        verus_dir: CargoVerus::default_verus_dir(),
        timeout: Duration::from_secs(timeout),
        battle_timeout: Duration::from_secs(battle_timeout),
    };
    match run(&options) {
        Ok((out, ev)) => {
            if !ev.validation.ok {
                println!("refused before verification:");
                ev.validation
                    .problems
                    .iter()
                    .for_each(|p| println!("  - {p}"));
            }
            println!("build: {}", ev.build.status);
            for p in &ev.verification.properties {
                println!(
                    "{:<9} {:<13} {}",
                    p.id,
                    format!("{:?}", p.status).to_lowercase(),
                    p.name
                );
            }
            for b in &ev.battles {
                match b.status {
                    "ok" => println!(
                        "battle ({}) vs {}: {} after {} turns; same implementation as verified: {}",
                        b.arena,
                        b.opponent,
                        b.winner
                            .as_deref()
                            .map_or("no winner".to_string(), |w| format!(
                                "{w} wins"
                            )),
                        b.turns.unwrap_or(0),
                        b.same_artifact.unwrap_or(false)
                    ),
                    s => println!("battle ({}): {s}", b.arena),
                }
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
