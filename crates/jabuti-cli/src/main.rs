mod config;
mod scan;
mod since;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use jabuti_core::report;

#[derive(Debug, Parser)]
#[command(
    name = "jabuti",
    version,
    about = "Code sensors and gates for AI agent harnesses"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Check {
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,

        #[arg(long, value_name = "REF")]
        since: Option<String>,

        #[arg(long, value_enum, default_value_t = Format::Agent)]
        format: Format,

        #[arg(long, default_value_t = report::DEFAULT_LIMIT)]
        limit: usize,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    Agent,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("jabuti: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    let Command::Check {
        paths,
        since,
        format,
        limit,
    } = Cli::parse().command;

    let settings = config::load(&std::env::current_dir()?)?;
    let changes = since.as_deref().map(since::Changes::since).transpose()?;
    let outcome = scan::scan(&paths, &settings, changes.as_ref())?;

    for path in &outcome.unreadable {
        eprintln!("jabuti: could not analyse {path}");
    }

    let Format::Agent = format;
    print!(
        "{}",
        report::agent(&outcome.findings, outcome.scanned, limit)
    );

    if report::has_errors(&outcome.findings) {
        return Ok(ExitCode::from(1));
    }

    Ok(ExitCode::SUCCESS)
}
