use std::collections::BTreeMap;

use tree_sitter::{Node, Query, QueryMatch};

use super::lang::{self, Table};
use crate::catalog::Concept;
use crate::lang::LanguageId;
use crate::model::Span;
use crate::policy::ConceptBindings;
use crate::syntax::{self, Parsed};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Occurrence {
    pub concept: Concept,
    pub subject: String,
    pub span: Span,
    pub in_test: bool,
}

struct Configured<'a> {
    language: LanguageId,
    separator: &'static str,
    bindings: &'a ConceptBindings,
    aliases: &'a BTreeMap<String, String>,
}

pub fn occurrences(
    parsed: &Parsed<'_>,
    bindings: &ConceptBindings,
    aliases: &BTreeMap<String, String>,
) -> Vec<Occurrence> {
    let table = lang::table(parsed.language());
    let configured = (!bindings.is_empty_for(parsed.language())).then_some(Configured {
        language: parsed.language(),
        separator: table.id.spec().path_separator,
        bindings,
        aliases,
    });
    let mut found = Vec::new();

    parsed.for_each_match(&table.queries().concepts, |matched, query| {
        if let Some((occurrence, node)) = captured_occurrence(matched, query, parsed.source()) {
            let in_test = inside_test(node, parsed.source(), table);
            found.push(Occurrence {
                in_test,
                ..occurrence
            });
        }

        if let Some(configured) = configured.as_ref()
            && let Some(call) = captured_call(matched, query)
        {
            let in_test = inside_test(call, parsed.source(), table);
            found.extend(configured.occurrences(call, parsed.source(), in_test));
        }
    });

    found.sort_by(|left, right| {
        left.span
            .cmp(&right.span)
            .then(left.concept.cmp(&right.concept))
            .then(left.subject.cmp(&right.subject))
    });
    found.dedup();
    found
}

fn captured_occurrence<'tree>(
    matched: &QueryMatch<'_, 'tree>,
    query: &Query,
    source: &str,
) -> Option<(Occurrence, Node<'tree>)> {
    let mut concept = None;
    let mut subject = None;
    let mut node = None;

    for capture in matched.captures {
        let label = query.capture_names()[capture.index as usize];
        if label == "subject" {
            subject = Some(capture.node);
        } else if let Some(captured) = concept_of(label) {
            concept = Some(captured);
            node = Some(capture.node);
        }
    }

    let subject = subject?;
    Some((
        Occurrence {
            concept: concept?,
            subject: syntax::text_of(subject, source),
            span: syntax::span_of(subject),
            in_test: false,
        },
        node?,
    ))
}

fn captured_call<'tree>(matched: &QueryMatch<'_, 'tree>, query: &Query) -> Option<Node<'tree>> {
    matched.captures.iter().find_map(|capture| {
        (query.capture_names()[capture.index as usize] == "call").then_some(capture.node)
    })
}

impl Configured<'_> {
    fn occurrences(&self, call: Node<'_>, source: &str, in_test: bool) -> Vec<Occurrence> {
        let written = syntax::text_of(call, source);
        let resolved = resolve(&written, self.separator, self.aliases);
        let subject = written
            .rsplit_once(self.separator)
            .map_or(written.as_str(), |(_, name)| name)
            .to_owned();

        Concept::ALL
            .into_iter()
            .filter(|concept| {
                self.bindings
                    .paths(self.language, *concept)
                    .is_some_and(|paths| {
                        paths.contains(&written)
                            || resolved.as_ref().is_some_and(|path| paths.contains(path))
                    })
            })
            .map(|concept| Occurrence {
                concept,
                subject: subject.clone(),
                span: syntax::span_of(call),
                in_test,
            })
            .collect()
    }
}

fn resolve(written: &str, separator: &str, aliases: &BTreeMap<String, String>) -> Option<String> {
    if let Some(path) = aliases.get(written) {
        return Some(path.clone());
    }

    let (first, rest) = written.split_once(separator)?;
    let prefix = aliases.get(first)?;

    Some(format!("{prefix}{separator}{rest}"))
}

pub(crate) fn concept_of(label: &str) -> Option<Concept> {
    let suffix = label.strip_prefix("concept.")?;

    match suffix {
        "error_discard" => Some(Concept::ErrorDiscard),
        "error_panic" => Some(Concept::ErrorPanic),
        "error_swallow" => Some(Concept::ErrorSwallow),
        "suppression" => Some(Concept::Suppression),
        "assertion" => Some(Concept::Assertion),
        unknown => panic!("query captures @concept.{unknown}, which maps to no concept"),
    }
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
    lang::markers(node, source, table, table.id.spec().test_markers)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSite {
    pub name: String,
    pub span: Span,
}

pub fn call_sites(parsed: &Parsed<'_>) -> Vec<CallSite> {
    let table = lang::table(parsed.language());
    let mut found = Vec::new();

    parsed.for_each_match(&table.queries().concepts, |matched, query| {
        if let Some(call) = captured_call(matched, query) {
            let name = syntax::text_of(call, parsed.source());
            if is_bare_identifier(&name) {
                found.push(CallSite {
                    name,
                    span: syntax::span_of(call),
                });
            }
        }
    });

    found
}

fn is_bare_identifier(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_alphanumeric() || c == '_')
}
