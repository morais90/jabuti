use std::collections::BTreeMap;

use crate::model::Finding;

const BEFORE: u32 = 1;
const AFTER: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub number: u32,
    pub text: String,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub hash: String,
    pub subject: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Trail {
    pub excerpts: BTreeMap<(String, u32), Vec<Line>>,
    pub commits: Vec<Commit>,
}

pub fn units(findings: &[Finding], limit: usize) -> Vec<(&str, u32)> {
    let mut units: Vec<(&str, u32)> = findings
        .iter()
        .take(limit)
        .map(|finding| (finding.path.as_str(), finding.span.start_line))
        .collect();
    units.dedup();
    units
}

pub fn excerpt(text: &str, start: u32, changed: impl Fn(u32) -> bool) -> Vec<Line> {
    let first = start.saturating_sub(BEFORE).max(1);
    let last = start + AFTER;

    text.lines()
        .zip(1..)
        .skip_while(|(_, number)| *number < first)
        .take_while(|(_, number)| *number <= last)
        .map(|(text, number)| Line {
            number,
            text: text.to_owned(),
            changed: changed(number),
        })
        .collect()
}

pub fn commits(log: &str) -> Vec<Commit> {
    log.lines()
        .filter_map(|line| line.split_once('\0'))
        .map(|(hash, subject)| Commit {
            hash: hash.to_owned(),
            subject: subject.to_owned(),
        })
        .collect()
}
