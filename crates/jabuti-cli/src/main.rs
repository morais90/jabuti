mod check;
mod config;
mod inputs;

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use jabuti_core::catalog::Rule;
use jabuti_core::{lang, report};

use crate::inputs::workspace;

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
    Languages,

    Tools {
        #[command(subcommand)]
        action: Option<ToolsAction>,
    },

    Check {
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,

        #[arg(long, value_name = "REF")]
        since: Option<String>,

        #[arg(long, value_enum)]
        format: Option<Format>,

        #[arg(long, default_value_t = report::DEFAULT_LIMIT)]
        limit: usize,
    },
}

#[derive(Debug, Subcommand)]
enum ToolsAction {
    Install,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    Human,
    Agent,
    Json,
    Measures,
    Sarif,
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
    match Cli::parse().command {
        Command::Languages => Ok(list_languages()),
        Command::Tools { action: None } => list_tools(),
        Command::Tools {
            action: Some(ToolsAction::Install),
        } => install_tools(),
        Command::Check {
            paths,
            since,
            format,
            limit,
        } => check(&paths, since.as_deref(), format, limit),
    }
}

fn list_languages() -> ExitCode {
    for spec in lang::ALL {
        let extensions: Vec<String> = spec
            .extensions
            .iter()
            .map(|extension| format!(".{extension}"))
            .collect();
        let available = jabuti_core::code::support::available_rules(spec.id).len();

        println!(
            "{:<10} {:<12} grammar {:<8} {available}/{} native rules",
            spec.id.name(),
            extensions.join(" "),
            spec.grammar_version,
            Rule::ALL.len(),
        );
    }

    ExitCode::SUCCESS
}

fn list_tools() -> Result<ExitCode> {
    let (_, settings) = workspace::discover()?;
    inputs::tools::known(&settings)?;
    let root = std::env::current_dir()?;

    for tool in inputs::tools::ALL {
        let note = match tool.status(&root, inputs::tools::enabled(&settings, tool.name)) {
            inputs::tools::Status::NotApplicable => "not applicable here".to_owned(),
            inputs::tools::Status::Unavailable => "install with `jabuti tools install`".to_owned(),
            inputs::tools::Status::Disabled => {
                format!("enable with [tools.{}] enabled = true", tool.name)
            }
            inputs::tools::Status::Ready => "will run".to_owned(),
        };

        println!("{:<16} {note}", tool.name);
    }

    Ok(ExitCode::SUCCESS)
}

fn install_tools() -> Result<ExitCode> {
    let (_, settings) = workspace::discover()?;
    inputs::tools::known(&settings)?;
    let root = std::env::current_dir()?;
    let installed = inputs::tools::install(&root)?;

    if installed.is_empty() {
        println!("No tools need installation.");
    } else {
        for name in installed {
            println!("Installed {name}.");
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn check(
    roots: &[PathBuf],
    since: Option<&str>,
    format: Option<Format>,
    limit: usize,
) -> Result<ExitCode> {
    let mut notices = Vec::new();
    let verdict = check::verdict(roots, since, &mut notices);
    announce(notices);

    let (root, outcome) = verdict?;
    let term = std::env::var_os("TERM");
    let interactive = report::interactive(std::io::stdout().is_terminal(), term.as_deref());
    let no_color = std::env::var_os("NO_COLOR");
    let format = format.unwrap_or(if interactive {
        Format::Human
    } else {
        Format::Agent
    });
    let rhythm = match format {
        Format::Human => {
            inputs::history::rhythm(&root, &report::shown_files(&outcome.findings, limit))
        }
        Format::Agent | Format::Json | Format::Measures | Format::Sarif => None,
    };
    let request = Request {
        format,
        limit,
        view: report::View {
            since,
            color: report::colored(interactive, no_color.as_deref()),
            links: Some(&root),
            rhythm: rhythm.as_ref(),
        },
    };
    print!("{}", rendered(&request, &outcome));

    if report::has_errors(&outcome.findings) {
        return Ok(ExitCode::from(1));
    }

    Ok(ExitCode::SUCCESS)
}

fn announce(mut notices: Vec<String>) {
    notices.sort();
    notices.dedup();

    for notice in notices {
        eprintln!("jabuti: {notice}");
    }
}

struct Request<'a> {
    format: Format,
    limit: usize,
    view: report::View<'a>,
}

fn rendered(request: &Request<'_>, outcome: &report::Outcome) -> String {
    match request.format {
        Format::Human => report::human(
            &outcome.findings,
            &outcome.unreadable,
            outcome.scanned,
            request.limit,
            request.view,
        ),
        Format::Agent => report::agent(
            &outcome.findings,
            &outcome.unreadable,
            outcome.scanned,
            request.limit,
        ),
        Format::Json => report::json(&outcome.findings, &outcome.unreadable, outcome.scanned),
        Format::Measures => report::measures(&outcome.readings, &outcome.unreadable),
        Format::Sarif => report::sarif(&outcome.findings, &outcome.unreadable),
    }
}
