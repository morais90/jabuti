use std::ops::Range;

use tree_sitter::{Node, Query, QueryMatch};

use super::lang::{self, Table};
use crate::model::{Span, UnitKind};
use crate::syntax::{self, Parsed};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub name: Option<String>,
    pub kind: UnitKind,
    pub span: Span,
    pub bytes: Range<usize>,
    pub parameters: u32,
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
        children: Vec::new(),
    })
}

fn declared_parameters(node: Node<'_>, table: &Table) -> u32 {
    let mut cursor = node.walk();
    let declared = node
        .named_children(&mut cursor)
        .filter(|child| !table.implicit_parameters.contains(&child.kind()))
        .count();

    u32::try_from(declared).unwrap_or(u32::MAX)
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

    let mut open: Vec<Unit> = Vec::new();

    for unit in captured {
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
