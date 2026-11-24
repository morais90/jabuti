pub mod coverage;

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::catalog::RuleId;
use crate::model::{Detail, Finding, Span};

pub fn cargo_diagnostics(tool: &str, output: &str, asked: impl Fn(&str) -> bool) -> Vec<Finding> {
    let mut findings: Vec<Finding> = output
        .lines()
        .filter_map(|line| serde_json::from_str::<Line>(line).ok())
        .filter_map(|line| line.message)
        .filter(|message| to_fix(message, &asked))
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

pub fn enabled_by_default(listing: &str) -> BTreeSet<String> {
    listing
        .lines()
        .filter_map(|line| {
            let mut columns = line.split_whitespace();
            let name = columns.next()?;
            let level = columns.next()?;

            ON.contains(&level).then(|| name.replace('-', "_"))
        })
        .collect()
}

const ON: [&str; 3] = ["warn", "deny", "forbid"];

fn to_fix(message: &Message, asked: impl Fn(&str) -> bool) -> bool {
    match (message.level.as_str(), &message.code) {
        ("error", _) => true,
        ("warning", Some(code)) => asked(&code.id),
        _ => false,
    }
}

fn finding(tool: &str, message: &Message) -> Option<Finding> {
    let lint = message.code.as_ref().map(|code| lint_name(&code.id))?;
    let span = message.spans.iter().find(|span| span.is_primary)?;

    Some(Finding {
        rule: RuleId::External {
            tool: tool.to_owned(),
            lint,
        },
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
