use std::fmt::Write;

use serde::Serialize;

use crate::model::{Detail, Finding, Reading, Severity};

pub const DEFAULT_LIMIT: usize = 40;
pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Scanned {
    pub files: usize,
    pub units: usize,
}

pub fn agent(findings: &[Finding], scanned: Scanned, limit: usize) -> String {
    let mut rendered = String::new();
    let errors = count(findings, Severity::Error);
    let warnings = count(findings, Severity::Warning);

    if findings.is_empty() {
        writeln!(rendered, "No findings across {}.", files_and_units(scanned))
            .expect("writing to a string never fails");

        return rendered;
    }

    writeln!(
        rendered,
        "{} and {} across {}.",
        plural(errors, "error"),
        plural(warnings, "warning"),
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

    rendered
}

fn write_finding(rendered: &mut String, finding: &Finding) {
    let subject = finding
        .subject
        .as_ref()
        .map_or(String::new(), |name| format!("{name}  "));

    let detail = match &finding.detail {
        Detail::Threshold { measured, limit } => format!("measured {measured}, limit {limit}"),
        Detail::Message { message } => message.clone(),
    };

    writeln!(
        rendered,
        "{}:{}  {}  {}  {subject}{detail}",
        finding.path,
        finding.span.start_line,
        finding.severity.label(),
        finding.rule.id(),
    )
    .expect("writing to a string never fails");
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

fn count(findings: &[Finding], severity: Severity) -> usize {
    findings
        .iter()
        .filter(|finding| finding.severity == severity)
        .count()
}

pub fn has_errors(findings: &[Finding]) -> bool {
    findings
        .iter()
        .any(|finding| finding.severity == Severity::Error)
}

#[derive(Debug, Serialize)]
struct Report<'a> {
    schema: u32,
    summary: Summary,
    findings: &'a [Finding],
}

#[derive(Debug, Serialize)]
struct Summary {
    files: usize,
    units: usize,
    errors: usize,
    warnings: usize,
}

#[derive(Debug, Serialize)]
struct Measurements<'a> {
    schema: u32,
    measures: &'a [Reading],
}

pub fn json(findings: &[Finding], scanned: Scanned) -> String {
    let report = Report {
        schema: SCHEMA,
        summary: Summary {
            files: scanned.files,
            units: scanned.units,
            errors: count(findings, Severity::Error),
            warnings: count(findings, Severity::Warning),
        },
        findings,
    };

    rendered(&report)
}

pub fn measures(readings: &[Reading]) -> String {
    rendered(&Measurements {
        schema: SCHEMA,
        measures: readings,
    })
}

fn rendered<T: Serialize>(value: &T) -> String {
    let mut json = serde_json::to_string_pretty(value).expect("the report is serialisable");
    json.push('\n');
    json
}
