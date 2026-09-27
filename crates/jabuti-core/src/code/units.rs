use std::ops::Range;

use tree_sitter::{Node, Query, QueryMatch};

use super::lang::{self, Table};
use crate::catalog::UnitKind;
use crate::lang::LanguageId;
use crate::model::Span;
use crate::syntax::{self, Parsed};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub name: Option<String>,
    pub kind: UnitKind,
    pub span: Span,
    pub bytes: Range<usize>,
    pub parameters: u32,
    pub is_test: bool,
    pub should_panic: bool,
    pub children: Vec<Unit>,
}

pub fn units(parsed: &Parsed<'_>) -> Unit {
    let table = lang::table(parsed.language());
    let mut captured = Vec::new();

    parsed.for_each_match(&table.queries().units, |matched, query| {
        if let Some(unit) = captured_unit(matched, query, parsed.source(), table) {
            captured.push(unit);
        }
    });

    nest(captured, file_unit(parsed.source()))
}

pub(crate) fn measured_separately(kind: UnitKind) -> bool {
    matches!(
        kind,
        UnitKind::File | UnitKind::Module | UnitKind::Type | UnitKind::Function
    )
}

fn file_unit(source: &str) -> Unit {
    let lines = source.split_inclusive('\n').count().max(1);

    Unit {
        name: None,
        kind: UnitKind::File,
        span: Span {
            start_line: 1,
            end_line: u32::try_from(lines).unwrap_or(u32::MAX),
        },
        bytes: 0..source.len(),
        parameters: 0,
        is_test: false,
        should_panic: false,
        children: Vec::new(),
    }
}

fn captured_unit(
    matched: &QueryMatch<'_, '_>,
    query: &Query,
    source: &str,
    table: &Table,
) -> Option<Unit> {
    let mut kind = None;
    let mut node = None;
    let mut name = None;
    let mut parameters = 0;

    for capture in matched.captures {
        let label = query.capture_names()[capture.index as usize];
        if let Some(labelled_kind) = kind_of_label(label) {
            kind = Some(labelled_kind);
            node = Some(capture.node);
        } else if label == "name" {
            name = capture
                .node
                .utf8_text(source.as_bytes())
                .ok()
                .map(str::to_owned);
        } else if label == "parameters" {
            parameters = declared_parameters(capture.node, table);
        }
    }

    let node = node?;
    Some(Unit {
        name,
        kind: kind?,
        span: syntax::span_of(node),
        bytes: node.byte_range(),
        parameters,
        is_test: is_test_unit(node, source, table),
        should_panic: table.id == LanguageId::Rust
            && rust_attribute_names(node, source, table).any(|name| name == "should_panic"),
        children: Vec::new(),
    })
}

fn is_test_unit(node: Node<'_>, source: &str, table: &Table) -> bool {
    match table.id {
        LanguageId::TypeScript => is_test_callback(node, source),
        LanguageId::Rust => rust_attribute_names(node, source, table)
            .any(|name| table.test_entry_markers.contains(&name)),
        LanguageId::Kotlin => lang::markers(node, source, table, table.test_entry_markers),
    }
}

fn rust_attribute_names<'a>(
    node: Node<'_>,
    source: &'a str,
    table: &Table,
) -> impl Iterator<Item = &'a str> {
    lang::decorators(node, table)
        .into_iter()
        .filter_map(move |decorator| {
            let attribute = decorator.named_child(0)?;
            let path = attribute.named_child(0)?;
            let text = path.utf8_text(source.as_bytes()).ok()?;
            Some(text.rsplit("::").next().unwrap_or(text))
        })
}

fn is_test_callback(node: Node<'_>, source: &str) -> bool {
    let Some(arguments) = node.parent().filter(|parent| parent.kind() == "arguments") else {
        return false;
    };
    let Some(call) = arguments
        .parent()
        .filter(|parent| parent.kind() == "call_expression")
    else {
        return false;
    };
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };

    match function.kind() {
        "identifier" => is_test_runner(function, source),
        "member_expression" => {
            let object = function.child_by_field_name("object");
            let property = function.child_by_field_name("property");

            object.is_some_and(|node| is_test_runner(node, source))
                && property.is_some_and(|node| matches!(text_of(node, source), "only" | "skip"))
        }
        _ => false,
    }
}

fn is_test_runner(node: Node<'_>, source: &str) -> bool {
    matches!(text_of(node, source), "it" | "test")
}

fn text_of<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or_default()
}

fn declared_parameters(node: Node<'_>, table: &Table) -> u32 {
    if !table.parameter_containers.contains(&node.kind()) {
        return u32::from(!implicit_parameter(node, table));
    }

    let mut cursor = node.walk();
    let declared = node
        .named_children(&mut cursor)
        .filter(|child| !implicit_parameter(*child, table))
        .count();

    u32::try_from(declared).unwrap_or(u32::MAX)
}

fn implicit_parameter(node: Node<'_>, table: &Table) -> bool {
    table.implicit_parameters.contains(&node.kind())
        || node
            .child_by_field_name("pattern")
            .is_some_and(|pattern| table.implicit_parameter_patterns.contains(&pattern.kind()))
}

fn kind_of_label(label: &str) -> Option<UnitKind> {
    let suffix = label.strip_prefix("unit.")?;

    match suffix {
        "module" => Some(UnitKind::Module),
        "type" => Some(UnitKind::Type),
        "function" => Some(UnitKind::Function),
        "closure" => Some(UnitKind::Closure),
        unknown => panic!("query captures @unit.{unknown}, which maps to no unit kind"),
    }
}

fn nest(mut captured: Vec<Unit>, mut file: Unit) -> Unit {
    captured.sort_by(|left, right| {
        left.bytes
            .start
            .cmp(&right.bytes.start)
            .then(right.bytes.end.cmp(&left.bytes.end))
    });
    let mut unique = Vec::with_capacity(captured.len());
    for unit in captured {
        if unique
            .last()
            .is_some_and(|previous: &Unit| previous.bytes == unit.bytes)
        {
            continue;
        }
        unique.push(unit);
    }

    let mut open: Vec<Unit> = Vec::new();

    for unit in unique {
        while let Some(closed) = close_enclosing(&mut open, &unit.bytes) {
            attach(&mut open, &mut file, closed);
        }
        open.push(unit);
    }

    while let Some(remaining) = open.pop() {
        attach(&mut open, &mut file, remaining);
    }

    file
}

fn close_enclosing(open: &mut Vec<Unit>, bytes: &Range<usize>) -> Option<Unit> {
    let innermost = open.last()?;
    if innermost.bytes.start <= bytes.start && bytes.end <= innermost.bytes.end {
        return None;
    }
    open.pop()
}

fn attach(open: &mut [Unit], file: &mut Unit, unit: Unit) {
    match open.last_mut() {
        Some(parent) => parent.children.push(unit),
        None => file.children.push(unit),
    }
}
