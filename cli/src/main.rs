//! `cargo robocode`: write, battle and verify robocode robots locally.

mod template;

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
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(f) => {
            eprintln!("error: {}", f.message);
            ExitCode::from(f.code)
        }
    }
}
