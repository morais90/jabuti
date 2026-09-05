use std::collections::{BTreeMap, BTreeSet};

use tree_sitter::Node;

use super::lang;
use crate::model::Span;
use crate::syntax::{self, Parsed};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileFacts {
    pub module: String,
    pub declares: BTreeSet<String>,
    pub paths: BTreeMap<String, Span>,
    pub names: BTreeMap<String, Span>,
}

pub fn facts(parsed: &Parsed<'_>) -> FileFacts {
    let mut facts = FileFacts::default();

    parsed.for_each_match(lang::references(parsed.language()), |matched, query| {
        for capture in matched.captures {
            let name = query.capture_names()[capture.index as usize];
            record(&mut facts, name, capture.node, parsed.source());
        }
    });

    facts
}

fn record(facts: &mut FileFacts, capture: &str, node: Node<'_>, source: &str) {
    let at = syntax::span_of(node);

    match capture {
        "package" => facts.module = syntax::text_of(node, source),
        "declaration" => {
            facts.declares.insert(syntax::text_of(node, source));
        }
        "reference.name" => remember(&mut facts.names, syntax::text_of(node, source), at),
        "reference.path" => {
            if let Some(path) = widest_path(node, source) {
                remember(&mut facts.paths, path, at);
            }
        }
        "reference.token" => {
            if let Some(path) = token_path(node, source) {
                remember(&mut facts.paths, path, at);
            }
        }
        "reference.list" => {
            for path in list_paths(node, source) {
                remember(&mut facts.paths, path, at);
            }
        }
        _ => {}
    }
}

fn remember(seen: &mut BTreeMap<String, Span>, name: String, at: Span) {
    seen.entry(name).or_insert(at);
}

fn widest_path(node: Node<'_>, source: &str) -> Option<String> {
    let mut widest = node;
    while let Some(parent) = widest.parent() {
        if parent.kind() != widest.kind() {
            break;
        }
        widest = parent;
    }

    let covered_by_a_list = widest
        .parent()
        .is_some_and(|parent| matches!(parent.kind(), "use_list" | "scoped_use_list"));

    (!covered_by_a_list).then(|| syntax::text_of(widest, source))
}

fn list_paths(node: Node<'_>, source: &str) -> Vec<String> {
    if node
        .parent()
        .is_some_and(|parent| parent.kind() == "use_list")
    {
        return Vec::new();
    }

    expanded(node, source)
}

fn expanded(node: Node<'_>, source: &str) -> Vec<String> {
    let Some(prefix) = node.child_by_field_name("path") else {
        return Vec::new();
    };
    let Some(list) = node.child_by_field_name("list") else {
        return Vec::new();
    };

    let prefix = syntax::text_of(prefix, source);
    let mut cursor = list.walk();

    list.named_children(&mut cursor)
        .flat_map(|entry| leaves(entry, source))
        .map(|leaf| format!("{prefix}::{leaf}"))
        .collect()
}

fn leaves(entry: Node<'_>, source: &str) -> Vec<String> {
    match entry.kind() {
        "identifier" | "scoped_identifier" => vec![syntax::text_of(entry, source)],
        "use_as_clause" => entry
            .child_by_field_name("path")
            .map(|path| syntax::text_of(path, source))
            .into_iter()
            .collect(),
        "scoped_use_list" => expanded(entry, source),
        _ => Vec::new(),
    }
}

fn token_path(node: Node<'_>, source: &str) -> Option<String> {
    if node
        .prev_sibling()
        .is_some_and(|before| before.kind() == "::")
    {
        return None;
    }

    let mut path = syntax::text_of(node, source);
    let mut sibling = node.next_sibling();

    while let Some(separator) = sibling {
        if separator.kind() != "::" {
            break;
        }
        let Some(segment) = separator.next_sibling() else {
            break;
        };
        if segment.kind() != "identifier" {
            break;
        }
        path.push_str("::");
        path.push_str(&syntax::text_of(segment, source));
        sibling = segment.next_sibling();
    }

    path.contains("::").then_some(path)
}
