use std::collections::BTreeMap;

use crate::model::{Detail, Finding, Fragment, Rule, RuleId, Severity};
use crate::policy::Policy;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFragments {
    pub path: String,
    pub fragments: Vec<Fragment>,
}

pub fn duplicates(files: &[FileFragments], policy: &Policy) -> Vec<Finding> {
    let Some(config) = policy.config(Rule::DuplicateBlock) else {
        return Vec::new();
    };
    if config.severity == Severity::Off {
        return Vec::new();
    }

    let mut classes: BTreeMap<u64, Vec<Occurrence>> = BTreeMap::new();
    for file in files {
        for fragment in &file.fragments {
            classes
                .entry(fragment.hash)
                .or_default()
                .push(Occurrence::new(&file.path, fragment));
        }
    }

    let repeated: Vec<Vec<Occurrence>> = classes
        .into_values()
        .filter(|occurrences| occurrences.len() > 1)
        .collect();

    let mut findings: Vec<Finding> = widest(repeated)
        .into_iter()
        .map(|(occurrence, twins)| finding(&occurrence, &twins, config.severity, config.limit))
        .collect();

    findings.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.span.start_line.cmp(&right.span.start_line))
    });
    findings
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Occurrence {
    path: String,
    fragment: Fragment,
}

impl Occurrence {
    fn new(path: &str, fragment: &Fragment) -> Self {
        Self {
            path: path.to_owned(),
            fragment: fragment.clone(),
        }
    }

    fn encloses(&self, other: &Self) -> bool {
        self.path == other.path
            && self.fragment.bytes.start <= other.fragment.bytes.start
            && other.fragment.bytes.end <= self.fragment.bytes.end
    }
}

fn widest(classes: Vec<Vec<Occurrence>>) -> Vec<(Occurrence, Vec<Occurrence>)> {
    let mut reported: Vec<(Occurrence, Vec<Occurrence>)> = Vec::new();

    let mut ordered = classes;
    ordered.sort_by(|left, right| {
        right[0]
            .fragment
            .nodes
            .cmp(&left[0].fragment.nodes)
            .then(left[0].path.cmp(&right[0].path))
    });

    for class in ordered {
        for (index, occurrence) in class.iter().enumerate() {
            if reported.iter().any(|(kept, _)| kept.encloses(occurrence)) {
                continue;
            }

            let twins = class
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .map(|(_, twin)| twin.clone())
                .collect();

            reported.push((occurrence.clone(), twins));
        }
    }

    reported
}

fn finding(
    occurrence: &Occurrence,
    twins: &[Occurrence],
    severity: Severity,
    limit: u32,
) -> Finding {
    let where_else: Vec<String> = twins
        .iter()
        .map(|twin| format!("{}:{}", twin.path, twin.fragment.span.start_line))
        .collect();

    Finding {
        rule: RuleId::Native(Rule::DuplicateBlock),
        severity,
        path: occurrence.path.clone(),
        span: occurrence.fragment.span,
        subject: None,
        detail: Detail::Message {
            message: format!(
                "{} nodes repeated at {} (limit {limit})",
                occurrence.fragment.nodes,
                where_else.join(", ")
            ),
        },
    }
}
