mod code;
mod config;
mod git;
mod graph;
mod history;
mod project;
mod tools;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use jabuti_core::history::hotspot::{self, FileSummary};
use jabuti_core::model::Rule;
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

    Tools {
        #[command(subcommand)]
        action: Option<ToolsAction>,
    },

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

#[derive(Debug, Subcommand)]
enum ToolsAction {
    Install,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
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
    for spec in jabuti_core::lang::ALL {
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
    let (_, settings) = config::discover()?;
    tools::known(&settings)?;
    let root = std::env::current_dir()?;

    for tool in tools::ALL {
        let note = match tool.status(&root, tools::enabled(&settings, tool.name)) {
            tools::Status::NotApplicable => "not applicable here".to_owned(),
            tools::Status::Unavailable => "install with `jabuti tools install`".to_owned(),
            tools::Status::Disabled => {
                format!("enable with [tools.{}] enabled = true", tool.name)
            }
            tools::Status::Ready => "will run".to_owned(),
        };

        println!("{:<16} {note}", tool.name);
    }

    Ok(ExitCode::SUCCESS)
}

fn install_tools() -> Result<ExitCode> {
    let (_, settings) = config::discover()?;
    tools::known(&settings)?;
    let root = std::env::current_dir()?;
    let installed = tools::install(&root)?;

    if installed.is_empty() {
        println!("No tools need installation.");
    } else {
        for name in installed {
            println!("Installed {name}.");
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn check(roots: &[PathBuf], since: Option<&str>, format: Format, limit: usize) -> Result<ExitCode> {
    let (root, settings) = config::discover()?;
    tools::known(&settings)?;
    let changes = since
        .map(|reference| git::since::Changes::since(reference, &root))
        .transpose()?;
    let history = history::load(&settings);
    scope_notices(&settings, since.is_some());

    let paths = project::sources(roots, &settings.exclude, &root)?;
    let aliases = graph::aliases(&paths, &settings.concepts);
    let churn = history::commits(history.as_ref(), &paths);
    let request = code::Scan {
        project: &root,
        policy: &settings.policy,
        bindings: &settings.concepts,
        aliases: &aliases,
        changes: changes.as_ref(),
        churn: &churn,
    };
    let mut outcome = code::scan(&paths, &request);
    if changes.is_none() {
        outcome.findings.extend(hotspot::hotspots(
            &summaries(&outcome.measured),
            &settings.policy,
        ));
    }
    let here = std::env::current_dir()?;
    outcome.findings.extend(tools::findings(&tools::Scan {
        here: &here,
        project: &root,
        paths: &paths,
        settings: &settings,
        changes: changes.as_ref(),
    }));
    let (found, skipped) = graph::findings(&paths, &root, &settings, changes.as_ref())?;
    outcome.findings.extend(found);
    outcome.unreadable.extend(skipped);
    order(&mut outcome);

    print!("{}", rendered(format, &outcome, limit));

    if report::has_errors(&outcome.findings) {
        return Ok(ExitCode::from(1));
    }

    Ok(ExitCode::SUCCESS)
}

fn rendered(format: Format, outcome: &code::Outcome, limit: usize) -> String {
    match format {
        Format::Agent => report::agent(
            &outcome.findings,
            &outcome.unreadable,
            outcome.scanned,
            limit,
        ),
        Format::Json => report::json(&outcome.findings, &outcome.unreadable, outcome.scanned),
        Format::Measures => report::measures(&outcome.readings, &outcome.unreadable),
        Format::Sarif => report::sarif(&outcome.findings, &outcome.unreadable),
    }
}

fn order(outcome: &mut code::Outcome) {
    outcome
        .unreadable
        .sort_by(|left, right| left.path.cmp(&right.path));
    outcome
        .unreadable
        .dedup_by(|left, right| left.path == right.path);
    outcome.findings.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.span.start_line.cmp(&right.span.start_line))
    });
}

fn scope_notices(settings: &config::Settings, scoped: bool) {
    if scoped && settings.enabled(Rule::Hotspot) {
        eprintln!("jabuti: hotspot ranks a whole repository, so it is not evaluated with --since");
    }
    for rule in [
        Rule::NewDependency,
        Rule::SpeculativeApi,
        Rule::UncoveredNewCode,
    ] {
        if !scoped && settings.gates(rule) {
            eprintln!(
                "jabuti: {} compares against an earlier revision, so it needs --since",
                rule.id()
            );
        }
    }
}

fn summaries(measured: &[code::Measured]) -> Vec<FileSummary> {
    measured
        .iter()
        .map(|file| FileSummary {
            path: file.path.clone(),
            span: file.span,
            churn: file.churn,
            complexity: file.complexity,
        })
        .collect()
}
