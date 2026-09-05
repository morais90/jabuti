use serde::Deserialize;

use crate::model::{Detail, Finding, RuleId, Severity, Span};

pub fn cargo_diagnostics(tool: &str, output: &str) -> Vec<Finding> {
    let mut findings: Vec<Finding> = output
        .lines()
        .filter_map(|line| serde_json::from_str::<Line>(line).ok())
        .filter_map(|line| line.message)
        .filter_map(|message| finding(tool, &message))
        .collect();

    findings.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.span.start_line.cmp(&right.span.start_line))
            .then(left.rule.cmp(&right.rule))
    });
    findings.dedup();
    findings
}

fn finding(tool: &str, message: &Message) -> Option<Finding> {
    let severity = severity(&message.level)?;
    let lint = message.code.as_ref().map(|code| lint_name(&code.id))?;
    let span = message.spans.iter().find(|span| span.is_primary)?;

    Some(Finding {
        rule: RuleId::External {
            tool: tool.to_owned(),
            lint,
        },
        severity,
        path: span.file_name.clone(),
        span: Span {
            start_line: span.line_start,
            end_line: span.line_end,
        },
        subject: None,
        detail: Detail::Message {
            message: message.text.clone(),
        },
    })
}

fn severity(level: &str) -> Option<Severity> {
    match level {
        "error" => Some(Severity::Error),
        "warning" => Some(Severity::Warning),
        _ => None,
    }
}

fn lint_name(code: &str) -> String {
    code.split_once("::")
        .map_or(code, |(_, name)| name)
        .to_owned()
}

#[derive(Debug, Deserialize)]
struct Line {
    message: Option<Message>,
}

#[derive(Debug, Deserialize)]
struct Message {
    level: String,
    #[serde(rename = "message")]
    text: String,
    code: Option<Code>,
    #[serde(default)]
    spans: Vec<Region>,
}

#[derive(Debug, Deserialize)]
struct Code {
    #[serde(rename = "code")]
    id: String,
}

#[derive(Debug, Deserialize)]
struct Region {
    file_name: String,
    line_start: u32,
    line_end: u32,
    is_primary: bool,
}
