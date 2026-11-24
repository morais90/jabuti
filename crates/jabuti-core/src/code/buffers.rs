use tree_sitter::Node;

use super::lang::TextBuffers;
use crate::syntax;

pub(super) fn holds_text(target: Node<'_>, source: &str, buffers: &TextBuffers) -> bool {
    let name = syntax::text_of(target, source);
    let mut current = target;
    loop {
        if let Some(declared) = declared_before(current, &name, source, buffers) {
            return declared;
        }
        let Some(parent) = current.parent() else {
            return false;
        };
        if parent.kind() == buffers.scope {
            return parameter_of(parent, &name, source, buffers);
        }
        current = parent;
    }
}

fn declared_before(
    node: Node<'_>,
    name: &str,
    source: &str,
    buffers: &TextBuffers,
) -> Option<bool> {
    let mut sibling = node.prev_named_sibling();
    while let Some(candidate) = sibling {
        if let Some(declared) = declaration_of(candidate, name, source, buffers) {
            return Some(declared);
        }
        sibling = candidate.prev_named_sibling();
    }

    None
}

fn parameter_of(scope: Node<'_>, name: &str, source: &str, buffers: &TextBuffers) -> bool {
    scope
        .child_by_field_name("parameters")
        .and_then(|parameters| {
            let mut cursor = parameters.walk();
            parameters
                .named_children(&mut cursor)
                .find_map(|parameter| declaration_of(parameter, name, source, buffers))
        })
        .unwrap_or(false)
}

fn declaration_of(node: Node<'_>, name: &str, source: &str, buffers: &TextBuffers) -> Option<bool> {
    if !buffers.declarations.contains(&node.kind()) {
        return None;
    }
    let pattern = node.child_by_field_name("pattern")?;
    let written = syntax::text_of(pattern, source);
    if written.strip_prefix("mut ").unwrap_or(&written).trim() != name {
        return None;
    }

    let typed = node
        .child_by_field_name("type")
        .is_some_and(|written| type_name(&syntax::text_of(written, source)) == buffers.type_name);
    let built = node
        .child_by_field_name("value")
        .is_some_and(|value| built_by(value, source, buffers));

    Some(typed || built)
}

fn built_by(value: Node<'_>, source: &str, buffers: &TextBuffers) -> bool {
    buffers.builder_fields.iter().any(|field| {
        value.child_by_field_name(field).is_some_and(|called| {
            buffers
                .builders
                .contains(&syntax::text_of(called, source).as_str())
        })
    })
}

fn type_name(written: &str) -> &str {
    let mut rest = written.trim_start_matches('&').trim_start();
    if rest.starts_with('\'') {
        rest = rest.split_once(' ').map_or("", |(_, after)| after);
    }
    let rest = rest.strip_prefix("mut ").unwrap_or(rest).trim();

    rest.rsplit("::").next().unwrap_or(rest)
}
