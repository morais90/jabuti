mod human;
mod sarif;
pub mod trail;

use std::fmt::Write;
use std::path::Path;

use serde::Serialize;

pub use self::human::{View, colored, interactive};
use crate::model::{Detail, Finding, Reading, Unreadable};

pub const DEFAULT_LIMIT: usize = 40;
pub const WEEKS: usize = 12;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rhythm {
    pub weeks: [u32; WEEKS],
}

impl Rhythm {
    pub fn commits(&self) -> u32 {
        self.weeks.iter().sum()
    }
}
pub const SCHEMA: u32 = 5;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Scanned {
    pub files: usize,
    pub units: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub findings: Vec<Finding>,
    pub context: Vec<Finding>,
    pub readings: Vec<Reading>,
    pub scanned: Scanned,
    pub unreadable: Vec<Unreadable>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Judgement<'a> {
    pub findings: &'a [Finding],
    pub context: &'a [Finding],
    pub unreadable: &'a [Unreadable],
    pub scanned: Scanned,
}

impl Outcome {
    pub fn judgement(&self) -> Judgement<'_> {
        Judgement {
            findings: &self.findings,
            context: &self.context,
            unreadable: &self.unreadable,
            scanned: self.scanned,
        }
    }

    pub fn set_context_apart(&mut self) {
        let (findings, context): (Vec<Finding>, Vec<Finding>) = std::mem::take(&mut self.findings)
            .into_iter()
            .partition(|finding| finding.rule.fails());
        self.findings = findings;
        self.context.extend(context);
    }

    pub fn order(&mut self) {
        self.findings.sort();
        self.context.sort();
        self.readings
            .sort_by(|left, right| Path::new(&left.path).cmp(Path::new(&right.path)));
        self.unreadable
            .sort_by(|left, right| left.path.cmp(&right.path));
    }
}

pub fn agent(judgement: Judgement<'_>, limit: usize) -> String {
    let Judgement {
        findings,
        unreadable,
        scanned,
        ..
    } = judgement;
    let mut rendered = String::new();
    if findings.is_empty() {
        writeln!(rendered, "No findings across {}.", files_and_units(scanned))
            .expect("writing to a string never fails");
        write_context(&mut rendered, judgement.context, limit);
        rendered.push_str(&self::unreadable(unreadable, limit));

        return rendered;
    }

    writeln!(
        rendered,
        "{} to fix across {}.",
        plural(findings.len(), "finding"),
        files_and_units(scanned)
    )
    .expect("writing to a string never fails");
    rendered.push('\n');

    for finding in findings.iter().take(limit) {
        write_finding(&mut rendered, finding);
    }

    let hidden = findings.len().saturating_sub(limit);
    if hidden > 0 {
        writeln!(
            rendered,
            "\n{hidden} further findings not shown. Narrow the scope with a path argument."
        )
        .expect("writing to a string never fails");
    }

    write_context(&mut rendered, judgement.context, limit);
    rendered.push_str(&self::unreadable(unreadable, limit));

    rendered
}

fn write_context(rendered: &mut String, context: &[Finding], limit: usize) {
    if context.is_empty() {
        return;
    }

    rendered.push_str("\nContext, which does not fail the run:\n");
    for finding in context.iter().take(limit) {
        write_finding(rendered, finding);
    }

    let hidden = context.len().saturating_sub(limit);
    if hidden > 0 {
        let withheld = format!("{} not shown.\n", plural(hidden, "further note"));
        rendered.push_str(&withheld);
    }
}

fn unreadable(unreadable: &[Unreadable], limit: usize) -> String {
    let mut rendered = String::new();
    if unreadable.is_empty() {
        return rendered;
    }

    writeln!(rendered, "\n{}", not_measured(unreadable.len()))
        .expect("writing to a string never fails");

    for file in unreadable.iter().take(limit) {
        writeln!(rendered, "{}  {}", file.path, file.reason)
            .expect("writing to a string never fails");
    }

    let hidden = unreadable.len().saturating_sub(limit);
    if hidden > 0 {
        writeln!(rendered, "{} not shown.", plural(hidden, "further file"))
            .expect("writing to a string never fails");
    }

    rendered
}

fn not_measured(count: usize) -> String {
    if count == 1 {
        "1 file was not measured, so nothing above accounts for it.".to_owned()
    } else {
        format!("{count} files were not measured, so nothing above accounts for them.")
    }
}

fn write_finding(rendered: &mut String, finding: &Finding) {
    write!(
        rendered,
        "{}:{}  {}  ",
        finding.path,
        finding.span.start_line,
        finding.rule.id(),
    )
    .expect("writing to a string never fails");
    if let Some(subject) = &finding.subject {
        write!(rendered, "{subject}  ").expect("writing to a string never fails");
    }
    write_detail(rendered, &finding.detail);
    rendered.push('\n');
}

fn write_detail(rendered: &mut String, detail: &Detail) {
    match detail {
        Detail::Threshold { measured, limit } => {
            write!(rendered, "measured {measured}, limit {limit}")
                .expect("writing to a string never fails");
        }
        Detail::Message { message } => rendered.push_str(message),
    }
}

fn files_and_units(scanned: Scanned) -> String {
    format!(
        "{} and {}",
        plural(scanned.files, "file"),
        plural(scanned.units, "unit")
    )
}

fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("{count} {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

pub fn shown_files<'a>(judgement: &Judgement<'a>, limit: usize) -> Vec<&'a str> {
    let mut paths: Vec<&str> = judgement
        .findings
        .iter()
        .take(limit)
        .chain(judgement.context.iter().take(limit))
        .map(|finding| finding.path.as_str())
        .collect();
    paths.sort_unstable();
    paths.dedup();
    paths
}

pub fn fails(findings: &[Finding]) -> bool {
    !findings.is_empty()
}

#[derive(Debug, Serialize)]
struct Report<'a> {
    schema: u32,
    summary: Summary,
    findings: &'a [Finding],
    context: &'a [Finding],
    unreadable: &'a [Unreadable],
}

#[derive(Debug, Serialize)]
struct Summary {
    files: usize,
    units: usize,
    findings: usize,
    context: usize,
    unreadable: usize,
}

#[derive(Debug, Serialize)]
struct Measurements<'a> {
    schema: u32,
    measures: &'a [Reading],
    unreadable: &'a [Unreadable],
}

pub fn json(judgement: Judgement<'_>) -> String {
    let report = Report {
        schema: SCHEMA,
        summary: Summary {
            files: judgement.scanned.files,
            units: judgement.scanned.units,
            findings: judgement.findings.len(),
            context: judgement.context.len(),
            unreadable: judgement.unreadable.len(),
        },
        findings: judgement.findings,
        context: judgement.context,
        unreadable: judgement.unreadable,
    };

    rendered(&report)
}

pub fn human(judgement: Judgement<'_>, limit: usize, view: View<'_>) -> String {
    human::render(judgement, limit, view)
}

pub fn sarif(judgement: Judgement<'_>) -> String {
    sarif::render(judgement)
}

pub fn measures(readings: &[Reading], unreadable: &[Unreadable]) -> String {
    rendered(&Measurements {
        schema: SCHEMA,
        measures: readings,
        unreadable,
    })
}

fn rendered<T: Serialize>(value: &T) -> String {
    let mut json = serde_json::to_string_pretty(value).expect("the report is serialisable");
    json.push('\n');
    json
}
