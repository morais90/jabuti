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
pub const SCHEMA: u32 = 4;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Scanned {
    pub files: usize,
    pub units: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub findings: Vec<Finding>,
    pub readings: Vec<Reading>,
    pub scanned: Scanned,
    pub unreadable: Vec<Unreadable>,
}

impl Outcome {
    pub fn order(&mut self) {
        self.findings.sort();
        self.readings
            .sort_by(|left, right| Path::new(&left.path).cmp(Path::new(&right.path)));
        self.unreadable
            .sort_by(|left, right| left.path.cmp(&right.path));
    }
}

pub fn agent(
    findings: &[Finding],
    unreadable: &[Unreadable],
    scanned: Scanned,
    limit: usize,
) -> String {
    let mut rendered = String::new();
    if findings.is_empty() {
        writeln!(rendered, "No findings across {}.", files_and_units(scanned))
            .expect("writing to a string never fails");
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

    rendered.push_str(&self::unreadable(unreadable, limit));

    rendered
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

pub fn shown_files(findings: &[Finding], limit: usize) -> Vec<&str> {
    let mut paths: Vec<&str> = findings
        .iter()
        .take(limit)
        .map(|finding| finding.path.as_str())
        .collect();
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
    unreadable: &'a [Unreadable],
}

#[derive(Debug, Serialize)]
struct Summary {
    files: usize,
    units: usize,
    findings: usize,
    unreadable: usize,
}

#[derive(Debug, Serialize)]
struct Measurements<'a> {
    schema: u32,
    measures: &'a [Reading],
    unreadable: &'a [Unreadable],
}

pub fn json(findings: &[Finding], unreadable: &[Unreadable], scanned: Scanned) -> String {
    let report = Report {
        schema: SCHEMA,
        summary: Summary {
            files: scanned.files,
            units: scanned.units,
            findings: findings.len(),
            unreadable: unreadable.len(),
        },
        findings,
        unreadable,
    };

    rendered(&report)
}

pub fn human(
    findings: &[Finding],
    unreadable: &[Unreadable],
    scanned: Scanned,
    limit: usize,
    view: View<'_>,
) -> String {
    human::render(findings, unreadable, scanned, limit, view)
}

pub fn sarif(findings: &[Finding], unreadable: &[Unreadable]) -> String {
    sarif::render(findings, unreadable)
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
