use std::collections::{BTreeMap, BTreeSet};

use tree_sitter::{Node, Query, QueryMatch};

use super::lang::{self, Table};
use crate::model::Span;
use crate::syntax::{self, Parsed};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    pub span: Span,
    pub public: bool,
    pub marked: bool,
    pub owner: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileFacts {
    pub module: String,
    pub aliases: BTreeMap<String, String>,
    pub declares: BTreeMap<String, Vec<Declared>>,
    pub exports: BTreeSet<String>,
    pub glob_exports: BTreeSet<String>,
    pub paths: BTreeMap<String, Span>,
    pub names: BTreeMap<String, BTreeSet<Span>>,
    pub mentions: BTreeMap<String, BTreeSet<Span>>,
}

pub fn facts(parsed: &Parsed<'_>) -> FileFacts {
    let table = lang::table(parsed.language());
    let mut recorder = Recorder {
        facts: FileFacts::default(),
        source: parsed.source(),
        table,
    };

    parsed.for_each_match(table.references(), |matched, query| {
        recorder.record_match(matched, query);
    });

    recorder.facts
}

pub fn aliases(parsed: &Parsed<'_>) -> BTreeMap<String, String> {
    let table = lang::table(parsed.language());
    let mut found = BTreeMap::new();

    parsed.for_each_match(table.references(), |matched, query| {
        if let Some((local, canonical)) =
            captured_alias(matched, query, parsed.source(), table.path_separator)
        {
            found.entry(local).or_insert(canonical);
        }
    });

    found
}

fn captured_alias(
    matched: &QueryMatch<'_, '_>,
    query: &Query,
    source: &str,
    separator: &str,
) -> Option<(String, String)> {
    let mut path = None;
    let mut module = None;
    let mut name = None;
    let mut alias = None;

    for capture in matched.captures {
        let label = query.capture_names()[capture.index as usize];
        let text = || {
            syntax::text_of(capture.node, source)
                .trim_matches(['\'', '"'])
                .to_owned()
        };
        match label {
            "import.path" => path = Some(text()),
            "import.module" => module = Some(text()),
            "import.name" => name = Some(text()),
            "import.alias" => alias = Some(text()),
            _ => {}
        }
    }

    let canonical = path.or_else(|| {
        module.map(|module| {
            name.as_ref().map_or_else(
                || module.clone(),
                |name| format!("{module}{separator}{name}"),
            )
        })
    })?;
    let local = alias.or(name).unwrap_or_else(|| {
        canonical
            .rsplit_once(separator)
            .map_or(canonical.as_str(), |(_, name)| name)
            .to_owned()
    });

    Some((local, canonical))
}

struct Recorder<'a> {
    facts: FileFacts,
    source: &'a str,
    table: &'a Table,
}

impl Recorder<'_> {
    fn record_match(&mut self, matched: &QueryMatch<'_, '_>, query: &Query) {
        if let Some((local, canonical)) =
            captured_alias(matched, query, self.source, self.table.path_separator)
        {
            self.facts.aliases.entry(local).or_insert(canonical);
        }

        for capture in matched.captures {
            let label = query.capture_names()[capture.index as usize];
            if !label.starts_with("import.") {
                self.record(label, capture.node);
            }
        }
    }

    fn record(&mut self, capture: &str, node: Node<'_>) {
        match capture {
            "package" => self.facts.module = syntax::text_of(node, self.source),
            "declaration" => self.declare(node),
            "export.module" | "export.use" => self.export(capture, node),
            "reference.name" => self.name(node),
            "reference.mention" => self.mention(node),
            "reference.path" | "reference.token" | "reference.list" => self.path(capture, node),
            "reference.module" => self.module(node),
            _ => {}
        }
    }

    fn declare(&mut self, name: Node<'_>) {
        let Some(item) = name.parent() else {
            return;
        };
        let found = Declared {
            span: syntax::span_of(item),
            public: is_public(item, self.source, self.table),
            marked: is_marked(item, self.source, self.table),
            owner: owner_of(item, self.source, self.table),
        };

        self.facts
            .declares
            .entry(syntax::text_of(name, self.source))
            .or_default()
            .push(found);
    }

    fn export(&mut self, capture: &str, node: Node<'_>) {
        if !plainly_public(node) {
            return;
        }
        if capture == "export.module" {
            self.facts
                .exports
                .insert(syntax::text_of(node, self.source));
        } else {
            self.facts.exports.extend(reexported(node, self.source));
            self.facts
                .glob_exports
                .extend(glob_exported(node, self.source));
        }
    }

    fn name(&mut self, node: Node<'_>) {
        let at = syntax::span_of(node);
        self.facts
            .names
            .entry(syntax::text_of(node, self.source))
            .or_default()
            .insert(at);
    }

    fn mention(&mut self, node: Node<'_>) {
        if declares_here(node) {
            return;
        }
        let at = syntax::span_of(node);
        self.facts
            .mentions
            .entry(syntax::text_of(node, self.source))
            .or_default()
            .insert(at);
    }

    fn path(&mut self, capture: &str, node: Node<'_>) {
        let at = syntax::span_of(node);
        let paths = match capture {
            "reference.path" => widest_path(node, self.source).into_iter().collect(),
            "reference.token" => token_path(node, self.source).into_iter().collect(),
            _ => list_paths(node, self.source),
        };

        for path in paths {
            self.facts.paths.entry(path).or_insert(at);
        }
    }

    fn module(&mut self, node: Node<'_>) {
        let text = syntax::text_of(node, self.source);
        let path = text.trim_matches(['\'', '"']).to_owned();
        self.facts
            .paths
            .entry(path)
            .or_insert_with(|| syntax::span_of(node));
    }
}

fn is_public(item: Node<'_>, source: &str, table: &Table) -> bool {
    if wrapped_publicly(item, table) {
        return true;
    }

    let public_by_position = table.public_by_default
        || item
            .parent()
            .is_some_and(|parent| table.public_inside.contains(&parent.kind()));
    let visibility = modifiers_of(item, table)
        .into_iter()
        .find(|node| table.visibility_modifiers.contains(&node.kind()));

    match visibility {
        None => public_by_position,
        Some(node) if public_by_position => !table
            .restricting_modifiers
            .contains(&syntax::text_of(node, source).as_str()),
        Some(node) => table
            .public_modifiers
            .contains(&syntax::text_of(node, source).as_str()),
    }
}

fn wrapped_publicly(item: Node<'_>, table: &Table) -> bool {
    let mut current = item.parent();

    while let Some(parent) = current {
        if table.public_wrappers.contains(&parent.kind()) {
            return true;
        }
        if !table.public_transparents.contains(&parent.kind()) {
            return false;
        }
        current = parent.parent();
    }

    false
}

fn modifiers_of<'tree>(item: Node<'tree>, table: &Table) -> Vec<Node<'tree>> {
    let mut cursor = item.walk();
    let mut found = Vec::new();

    for child in item.children(&mut cursor) {
        if table.visibility_modifiers.contains(&child.kind()) {
            found.push(child);
        }
        if table.decorators_within.contains(&child.kind()) {
            let mut inner = child.walk();
            found.extend(child.children(&mut inner));
        }
    }

    found
}

fn is_marked(item: Node<'_>, source: &str, table: &Table) -> bool {
    decorated_beyond_inert(item, source, table) || inside_test_scope(item, source, table)
}

fn decorated_beyond_inert(item: Node<'_>, source: &str, table: &Table) -> bool {
    decorators_of(item, table).iter().any(|decorator| {
        let name = decorator_name(&syntax::text_of(*decorator, source));
        !table.inert_decorators.contains(&name.as_str())
    })
}

fn inside_test_scope(item: Node<'_>, source: &str, table: &Table) -> bool {
    let markers = table.id.spec().test_markers;
    let mut current = Some(item);

    while let Some(node) = current {
        let marked = decorators_of(node, table).iter().any(|decorator| {
            let text = syntax::text_of(*decorator, source);
            markers.iter().any(|mark| text.contains(mark))
        });
        if marked {
            return true;
        }
        current = node.parent();
    }

    false
}

fn decorators_of<'tree>(item: Node<'tree>, table: &Table) -> Vec<Node<'tree>> {
    let mut decorators = Vec::new();

    let mut sibling = item.prev_sibling();
    while let Some(current) = sibling {
        if table.decorators_before.contains(&current.kind()) {
            decorators.push(current);
        } else if !current.is_extra() {
            break;
        }
        sibling = current.prev_sibling();
    }

    let mut cursor = item.walk();
    decorators.extend(
        item.children(&mut cursor)
            .filter(|node| table.decorators_direct.contains(&node.kind())),
    );
    decorators.extend(
        modifiers_of(item, table)
            .into_iter()
            .filter(|node| node.kind() == table.annotation),
    );

    decorators
}

fn decorator_name(text: &str) -> String {
    let inside = text.trim_start_matches(['#', '!', '[', '@']);

    inside
        .split(['(', ']', '=', ' ', '<'])
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn owner_of(item: Node<'_>, source: &str, table: &Table) -> Option<String> {
    let body = item.parent()?;
    if !table.owner_bodies.contains(&body.kind()) {
        return None;
    }
    let owner = body.parent()?;
    if !table.owner_declarations.contains(&owner.kind()) {
        return None;
    }
    let mut name = owner.child_by_field_name(table.owner_field)?;
    while table.owner_name_wrappers.contains(&name.kind()) {
        name = name.child_by_field_name(table.owner_field)?;
    }

    Some(syntax::text_of(name, source))
}

fn plainly_public(name: Node<'_>) -> bool {
    let Some(item) = name.parent() else {
        return false;
    };
    let mut cursor = item.walk();

    item.children(&mut cursor)
        .find(|child| child.kind() == "visibility_modifier")
        .is_some_and(|visibility| visibility.named_child_count() == 0)
}

fn reexported(argument: Node<'_>, source: &str) -> Vec<String> {
    leaves(argument, source)
        .iter()
        .filter_map(|path| path.rsplit("::").next())
        .map(str::to_owned)
        .collect()
}

fn glob_exported(argument: Node<'_>, source: &str) -> Vec<String> {
    globs(argument, source)
        .iter()
        .filter_map(|path| {
            path.strip_suffix("::*")
                .or_else(|| path.strip_suffix("::self"))
                .map(str::to_owned)
        })
        .collect()
}

fn globs(node: Node<'_>, source: &str) -> Vec<String> {
    match node.kind() {
        "use_wildcard" | "self" => vec![syntax::text_of(node, source)],
        "scoped_use_list" => prefixed(node, source, globs),
        _ => Vec::new(),
    }
}

fn declares_here(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    if !DECLARING.contains(&parent.kind()) {
        return false;
    }

    ["name", "type"].iter().any(|field| {
        parent
            .child_by_field_name(field)
            .is_some_and(|named| named.id() == node.id())
    })
}

const DECLARING: [&str; 15] = [
    "struct_item",
    "enum_item",
    "union_item",
    "trait_item",
    "type_item",
    "function_item",
    "const_item",
    "static_item",
    "impl_item",
    "class_declaration",
    "object_declaration",
    "function_declaration",
    "type_alias",
    "variable_declarator",
    "public_field_definition",
];

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
    prefixed(node, source, leaves)
}

fn prefixed(
    node: Node<'_>,
    source: &str,
    entries: fn(Node<'_>, &str) -> Vec<String>,
) -> Vec<String> {
    let Some(prefix) = node.child_by_field_name("path") else {
        return Vec::new();
    };
    let Some(list) = node.child_by_field_name("list") else {
        return Vec::new();
    };

    let prefix = syntax::text_of(prefix, source);
    let mut cursor = list.walk();

    list.named_children(&mut cursor)
        .flat_map(|entry| entries(entry, source))
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
