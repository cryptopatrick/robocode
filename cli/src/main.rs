//! `cargo robocode`: write, battle and verify robocode robots locally.

mod battle;
mod build_project;
mod serve;
mod template;
mod verify;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cargo robocode", version, about = "Write, battle and verify robocode robots locally")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List the built-in robots you can battle against.
    List,
    /// Create <name>.rs from the robot template.
    New {
        /// File name without `.rs`, e.g. `hunter`.
        name: String,
    },
    /// Compile a robot file, battle it, and show the battle in your browser.
    Battle {
        /// Your robot file, e.g. `hunter.rs` (or a built-in robot name).
        robot: String,
        /// An opponent: a built-in robot name or a .rs file. Repeat for more (2-4 robots in total).
        #[arg(long = "vs", value_name = "OPPONENT")]
        vs: Vec<String>,
        /// Battlefield size.
        #[arg(long, default_value = "800x600")]
        arena: String,
        /// Turn limit (per round).
        #[arg(long, default_value_t = 5000)]
        turns: u64,
        /// Play several rounds and print each round's winner and the totals
        /// (the recording shows round 1).
        #[arg(long, default_value_t = 1)]
        rounds: u32,
        /// Also save the recording to this file.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Don't open a browser (print the address instead; with --output, just save and exit).
        #[arg(long)]
        no_open: bool,
        /// Show cargo's build output.
        #[arg(long)]
        verbose: bool,
        /// Run the battle live: it advances while you watch, at --tps turns per second.
        #[arg(long)]
        live: bool,
        /// Fight on the fixed urban map: buildings stop tanks and absorb bullets.
        #[arg(long)]
        urban: bool,
        /// Live speed: turns per second (1-1000) or "max".
        #[arg(long, default_value = "30")]
        tps: String,
        /// The robocode source to compile against.
        #[arg(long)]
        robocode_path: Option<PathBuf>,
    },
    /// Run Verus on a robot file (needs cargo verus on PATH).
    Verify {
        /// Your robot file.
        robot: String,
        /// The robocode source to verify against.
        #[arg(long)]
        robocode_path: Option<PathBuf>,
    },
}

/// A failure the user can act on: message plus exit code.
pub struct Failure {
    pub code: u8,
    pub message: String,
}

impl Failure {
    pub fn usage(message: impl Into<String>) -> Self {
        Failure { code: 2, message: message.into() }
    }
}

fn new_robot(name: &str) -> Result<(), Failure> {
    let stem = name.strip_suffix(".rs").unwrap_or(name);
    let path = PathBuf::from(format!("{stem}.rs"));
    if path.exists() {
        return Err(Failure::usage(format!("{} already exists", path.display())));
    }
    let file_stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    std::fs::write(&path, template::robot_file(&template::type_name(&file_stem)))
        .map_err(|e| Failure::usage(format!("could not write {}: {e}", path.display())))?;
    println!("Created {}. Battle it with: cargo robocode battle {}", path.display(), path.display());
    Ok(())
}

fn main() -> ExitCode {
    // `cargo robocode …` runs us as `cargo-robocode robocode …`.
    let mut args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("robocode") {
        args.remove(1);
    }
    let cli = Cli::parse_from(args);
    let result = match cli.command {
        Command::List => {
            for name in robocode::roster::BUILT_IN {
                println!("{name}");
            }
            Ok(())
        }
        Command::New { name } => new_robot(&name),
        Command::Battle { robot, vs, arena, turns, rounds, output, no_open, verbose, live, urban, tps, robocode_path } => {
            let tps = match tps.as_str() {
                "max" => Ok(None),
                t => t.parse::<u32>().ok().filter(|t| (1..=1000).contains(t)).map(Some).ok_or_else(|| {
                    Failure::usage(format!("--tps must be 1..=1000 or max, got {t}"))
                }),
            };
            match tps {
                Err(f) => Err(f),
                Ok(tps) => {
                    let save_only = no_open && output.is_some() && !live;
                    let args = battle::BattleArgs { robot, vs, arena, turns, rounds, live, urban, output, verbose, robocode_path };
                    if live {
                        battle::compile(&args).and_then(|c| serve::serve_live(&c, tps, args.output.clone(), no_open))
                    } else {
                        battle::run(&args).and_then(|json| if save_only { Ok(()) } else { serve::serve(json, no_open) })
                    }
                }
            }
        }
        Command::Verify { robot, robocode_path } => verify::run(&robot, robocode_path.as_deref()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(f) => {
            eprintln!("error: {}", f.message);
            ExitCode::from(f.code)
        }
    }
}
