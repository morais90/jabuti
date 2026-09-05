use tree_sitter::{Node, Query, QueryMatch};

use super::lang::{self, Table};
use crate::lang::LanguageId;
use crate::model::{Detail, Finding, Rule, RuleId, Severity, Span};
use crate::policy::Policy;
use crate::syntax::{self, Parsed};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskingKind {
    Panic,
    Discard,
    Swallow,
}

impl MaskingKind {
    pub fn consequence(self) -> &'static str {
        match self {
            Self::Panic => "the failure becomes a panic",
            Self::Discard => "the failure is dropped without being read",
            Self::Swallow => "the failure is caught and nothing happens",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Masking {
    pub kind: MaskingKind,
    pub construct: String,
    pub span: Span,
}

pub fn maskings(parsed: &Parsed<'_>) -> Vec<Masking> {
    let table = lang::table(parsed.language());
    let mut found = Vec::new();

    parsed.for_each_match(&table.queries().masking, |matched, query| {
        let captured = captured_masking(matched, query, parsed.source());
        if let Some((masking, node)) = captured
            && !inside_test(node, parsed.source(), table)
        {
            found.push(masking);
        }
    });

    found.sort_by_key(|masking| masking.span.start_line);
    found
}

fn inside_test(node: Node<'_>, source: &str, table: &Table) -> bool {
    let mut current = Some(node);

    while let Some(inner) = current {
        if markers_around(inner, source, table) {
            return true;
        }
        current = inner.parent();
    }

    false
}

fn markers_around(node: Node<'_>, source: &str, table: &Table) -> bool {
    let mut attached = Vec::new();

    let mut sibling = node.prev_sibling();
    while let Some(current) = sibling {
        if !table.decorators_before.contains(&current.kind()) {
            break;
        }
        attached.push(current);
        sibling = current.prev_sibling();
    }

    let mut cursor = node.walk();
    attached.extend(
        node.children(&mut cursor)
            .filter(|child| table.decorators_within.contains(&child.kind())),
    );

    attached.iter().any(|node| {
        node.utf8_text(source.as_bytes())
            .is_ok_and(|text| table.test_markers.iter().any(|mark| text.contains(mark)))
    })
}

fn captured_masking<'tree>(
    matched: &QueryMatch<'_, 'tree>,
    query: &Query,
    source: &str,
) -> Option<(Masking, Node<'tree>)> {
    let mut kind = None;
    let mut construct = None;
    let mut node = None;

    for capture in matched.captures {
        let label = query.capture_names()[capture.index as usize];
        if label == "construct" {
            construct = Some(capture.node);
        } else if let Some(labelled) = kind_of_mask(label) {
            kind = Some(labelled);
            node = Some(capture.node);
        }
    }

    let (node, named) = (node?, construct?);
    Some((
        Masking {
            kind: kind?,
            construct: named.utf8_text(source.as_bytes()).ok()?.to_owned(),
            span: syntax::span_of(named),
        },
        node,
    ))
}

fn kind_of_mask(label: &str) -> Option<MaskingKind> {
    let suffix = label.strip_prefix("mask.")?;

    match suffix {
        "panic" => Some(MaskingKind::Panic),
        "discard" => Some(MaskingKind::Discard),
        "swallow" => Some(MaskingKind::Swallow),
        unknown => panic!("query captures @mask.{unknown}, which maps to no masking kind"),
    }
}

pub fn findings(
    path: &str,
    language: LanguageId,
    maskings: &[Masking],
    policy: &Policy,
) -> Vec<Finding> {
    let Some(config) = policy.config_for(language, Rule::ErrorMasking) else {
        return Vec::new();
    };
    if config.severity == Severity::Off {
        return Vec::new();
    }

    maskings
        .iter()
        .map(|masking| Finding {
            rule: RuleId::Native(Rule::ErrorMasking),
            severity: config.severity,
            path: path.to_owned(),
            span: masking.span,
            subject: Some(masking.construct.clone()),
            detail: Detail::Message {
                message: masking.kind.consequence().to_owned(),
            },
        })
        .collect()
}
