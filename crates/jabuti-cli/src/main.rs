mod churn;
mod config;
mod git;
mod scan;
mod since;
mod tools;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use jabuti_core::model::{Rule, Severity};
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
    Languages,

    Tools,

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
    Json,
    Measures,
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

fn run_tools(
    root: &std::path::Path,
    settings: &config::Settings,
    changes: Option<&since::Changes>,
) -> Vec<jabuti_core::model::Finding> {
    let mut findings = Vec::new();

    for tool in tools::ALL {
        if !tool
            .status(root, enabled_tool(settings, tool.name))
            .runnable()
        {
            continue;
        }

        match tool.run(root) {
            Ok(reported) => findings.extend(admitted(reported, root, settings, changes)),
            Err(reason) => eprintln!("jabuti: {reason}"),
        }
    }

    findings
}

fn admitted(
    reported: Vec<jabuti_core::model::Finding>,
    root: &std::path::Path,
    settings: &config::Settings,
    changes: Option<&since::Changes>,
) -> Vec<jabuti_core::model::Finding> {
    reported
        .into_iter()
        .filter_map(|finding| settings.policy.admit(finding))
        .filter(|finding| in_scope(finding, root, changes))
        .collect()
}

fn in_scope(
    finding: &jabuti_core::model::Finding,
    root: &std::path::Path,
    changes: Option<&since::Changes>,
) -> bool {
    changes.is_none_or(|changes| changes.touches(&root.join(&finding.path), finding.span))
}

fn enabled(settings: &config::Settings, rule: Rule) -> bool {
    settings
        .policy
        .config(rule)
        .is_some_and(|config| config.severity != Severity::Off)
}

fn history_when_needed(settings: &config::Settings) -> Option<churn::Churn> {
    if !enabled(settings, Rule::Churn) && !enabled(settings, Rule::Hotspot) {
        return None;
    }

    match churn::Churn::of_repository() {
        Ok(history) => Some(history),
        Err(reason) => {
            eprintln!(
                "jabuti: churn and hotspot need a git repository, so they were not evaluated ({reason})"
            );
            None
        }
    }
}

fn run() -> Result<ExitCode> {
    match Cli::parse().command {
        Command::Languages => Ok(list_languages()),
        Command::Tools => list_tools(),
        Command::Check {
            paths,
            since,
            format,
            limit,
        } => check(&paths, since.as_deref(), format, limit),
    }
}

fn list_languages() -> ExitCode {
    for spec in jabuti_core::lang::ALL {
        let extensions: Vec<String> = spec
            .extensions
            .iter()
            .map(|extension| format!(".{extension}"))
            .collect();

        println!(
            "{:<10} {:<12} grammar {}",
            spec.id.name(),
            extensions.join(" "),
            spec.grammar_version
        );
    }

    ExitCode::SUCCESS
}

fn list_tools() -> Result<ExitCode> {
    let root = std::env::current_dir()?;
    let settings = config::load(&root)?;

    for tool in tools::ALL {
        let note = match tool.status(&root, enabled_tool(&settings, tool.name)) {
            tools::Status::NotApplicable => "not applicable here".to_owned(),
            tools::Status::Unavailable => format!("install with `{}`", tool.install_hint),
            tools::Status::Disabled => {
                format!("enable with [tools.{}] enabled = true", tool.name)
            }
            tools::Status::Ready => "will run".to_owned(),
        };

        println!("{:<10} {note}", tool.name);
    }

    Ok(ExitCode::SUCCESS)
}

fn enabled_tool(settings: &config::Settings, name: &str) -> bool {
    settings.tools.get(name).copied().unwrap_or(false)
}

fn check(paths: &[PathBuf], since: Option<&str>, format: Format, limit: usize) -> Result<ExitCode> {
    let root = std::env::current_dir()?;
    let settings = config::load(&root)?;
    let changes = since.map(since::Changes::since).transpose()?;
    let history = history_when_needed(&settings);
    if changes.is_some() && enabled(&settings, Rule::Hotspot) {
        eprintln!("jabuti: hotspot ranks a whole repository, so it is not evaluated with --since");
    }

    let mut outcome = scan::scan(paths, &settings, changes.as_ref(), history.as_ref())?;
    outcome
        .findings
        .extend(run_tools(&root, &settings, changes.as_ref()));
    outcome.findings.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.span.start_line.cmp(&right.span.start_line))
    });

    for path in &outcome.unreadable {
        eprintln!("jabuti: could not analyse {path}");
    }

    print!(
        "{}",
        match format {
            Format::Agent => report::agent(&outcome.findings, outcome.scanned, limit),
            Format::Json => report::json(&outcome.findings, outcome.scanned),
            Format::Measures => report::measures(&outcome.readings),
        }
    );

    if report::has_errors(&outcome.findings) {
        return Ok(ExitCode::from(1));
    }

    Ok(ExitCode::SUCCESS)
}
