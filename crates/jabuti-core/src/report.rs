use std::fmt::Write;

use serde::Serialize;

use crate::model::{Detail, Finding, Reading, Severity, Unreadable};

pub const DEFAULT_LIMIT: usize = 40;
pub const SCHEMA: u32 = 2;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Scanned {
    pub files: usize,
    pub units: usize,
}

pub fn agent(
    findings: &[Finding],
    unreadable: &[Unreadable],
    scanned: Scanned,
    limit: usize,
) -> String {
    let mut rendered = String::new();
    let errors = count(findings, Severity::Error);
    let warnings = count(findings, Severity::Warning);

    if findings.is_empty() {
        writeln!(rendered, "No findings across {}.", files_and_units(scanned))
            .expect("writing to a string never fails");
        write_unreadable(&mut rendered, unreadable, limit);

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

    write_unreadable(&mut rendered, unreadable, limit);

    rendered
}

fn write_unreadable(rendered: &mut String, unreadable: &[Unreadable], limit: usize) {
    if unreadable.is_empty() {
        return;
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
}

fn not_measured(count: usize) -> String {
    if count == 1 {
        "1 file was not measured, so the verdict above does not cover it.".to_owned()
    } else {
        format!("{count} files were not measured, so the verdict above does not cover them.")
    }
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
    unreadable: &'a [Unreadable],
}

#[derive(Debug, Serialize)]
struct Summary {
    files: usize,
    units: usize,
    errors: usize,
    warnings: usize,
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
            errors: count(findings, Severity::Error),
            warnings: count(findings, Severity::Warning),
            unreadable: unreadable.len(),
        },
        findings,
        unreadable,
    };

    rendered(&report)
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
