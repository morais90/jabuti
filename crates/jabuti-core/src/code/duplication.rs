use std::collections::BTreeMap;
use std::ops::Range;

use tree_sitter::Node;

use super::lang;
use crate::model::{Detail, Finding, Rule, RuleId, Severity, Span};
use crate::policy::Policy;
use crate::syntax::{self, Parsed};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fragment {
    pub hash: u64,
    pub span: Span,
    pub bytes: Range<usize>,
    pub nodes: u32,
}

pub fn fragments(parsed: &Parsed<'_>, limit: u32) -> Vec<Fragment> {
    let mut found = Vec::new();
    let shape = Shape {
        limit,
        metadata: lang::table(parsed.language()).metadata_nodes,
    };
    shape.of(parsed.root(), &mut found);
    found.sort_by_key(|fragment| fragment.bytes.start);

    found
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn mixed(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |acc, byte| {
        (acc ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

struct Shape {
    limit: u32,
    metadata: &'static [&'static str],
}

impl Shape {
    fn of(&self, node: Node<'_>, found: &mut Vec<Fragment>) -> (u64, u32) {
        let mut hash = mixed(FNV_OFFSET, node.kind().as_bytes());
        let mut nodes = 1;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_extra() || self.metadata.contains(&child.kind()) {
                continue;
            }

            let (child_hash, child_nodes) = self.of(child, found);
            hash = mixed(hash, &child_hash.to_le_bytes());
            nodes += child_nodes;
        }

        if nodes > self.limit {
            found.push(Fragment {
                hash,
                span: syntax::span_of(node),
                bytes: node.byte_range(),
                nodes,
            });
        }

        (hash, nodes)
    }
}

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

#[derive(Debug)]
struct Twins {
    listed: Vec<Occurrence>,
    total: usize,
}

fn widest(classes: Vec<Vec<Occurrence>>) -> Vec<(Occurrence, Twins)> {
    let mut reported: Vec<(Occurrence, Twins)> = Vec::new();

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

            reported.push((occurrence.clone(), twins_of(&class, index)));
        }
    }

    reported
}

const TWINS_LISTED: usize = 3;

fn twins_of(class: &[Occurrence], index: usize) -> Twins {
    let listed = class
        .iter()
        .enumerate()
        .filter(|(other, _)| *other != index)
        .take(TWINS_LISTED)
        .map(|(_, twin)| twin.clone())
        .collect();

    Twins {
        listed,
        total: class.len() - 1,
    }
}

fn finding(occurrence: &Occurrence, twins: &Twins, severity: Severity, limit: u32) -> Finding {
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
                where_else(twins)
            ),
        },
    }
}

fn where_else(twins: &Twins) -> String {
    let listed = twins
        .listed
        .iter()
        .map(|twin| format!("{}:{}", twin.path, twin.fragment.span.start_line))
        .collect::<Vec<String>>()
        .join(", ");

    match twins.total.checked_sub(twins.listed.len()) {
        Some(remaining) if remaining > 0 => format!("{listed} and {remaining} more"),
        _ => listed,
    }
}
