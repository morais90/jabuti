use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use super::facts::{Declared, FileFacts};
use super::index::{Index, Source};
use crate::model::Span;

const ENTRY_POINT: &str = "main";
const CRATE_ROOT: &str = "lib.rs";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Roots {
    pub exported: BTreeSet<PathBuf>,
    pub surface: BTreeSet<String>,
}

pub fn roots(sources: &[Source], index: &Index) -> Roots {
    let mut roots = Roots::default();
    let mut queue: VecDeque<&Source> = sources
        .iter()
        .filter(|source| {
            source
                .path
                .file_name()
                .is_some_and(|name| name == CRATE_ROOT)
        })
        .collect();

    while let Some(source) = queue.pop_front() {
        if roots.exported.insert(source.path.clone()) {
            roots.surface.extend(source.facts.exports.iter().cloned());
            roots.surface.extend(source.facts.declares.keys().cloned());
            queue.extend(exported_children(source, sources, index));
        }
    }

    roots
}

fn exported_children<'a>(of: &Source, sources: &'a [Source], index: &Index) -> Vec<&'a Source> {
    let modules = of.facts.exports.iter().map(|name| vec![name.as_str()]);
    let globbed = of
        .facts
        .glob_exports
        .iter()
        .map(|path| path.split("::").collect::<Vec<&str>>());

    modules
        .chain(globbed)
        .filter_map(|segments| index.descendant_module(&of.path, &segments))
        .filter_map(|child| sources.iter().find(|candidate| candidate.path == child))
        .collect()
}

impl Roots {
    fn covers(&self, name: &str, declared: &Declared) -> bool {
        name == ENTRY_POINT
            || self.surface.contains(name)
            || declared
                .owner
                .as_ref()
                .is_some_and(|owner| self.surface.contains(owner))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Speculative {
    pub path: PathBuf,
    pub name: String,
    pub span: Span,
}

pub fn speculative(
    now: &Source,
    then: Option<&FileFacts>,
    all: &[Source],
    roots: &Roots,
) -> Vec<Speculative> {
    let mut found: Vec<Speculative> = now
        .facts
        .declares
        .iter()
        .filter(|(name, _)| !then.is_some_and(|facts| facts.declares.contains_key(*name)))
        .flat_map(|(name, declarations)| unreferenced(name, declarations, now, all, roots))
        .collect();

    found.sort_by_key(|item| item.span);
    found
}

fn unreferenced(
    name: &str,
    declarations: &[Declared],
    now: &Source,
    all: &[Source],
    roots: &Roots,
) -> Vec<Speculative> {
    let own: Vec<Span> = declarations.iter().map(|declared| declared.span).collect();

    declarations
        .iter()
        .filter(|declared| declared.public && !declared.marked)
        .filter(|declared| !roots.covers(name, declared))
        .filter(|_| !referenced(name, &now.path, &own, all))
        .map(|declared| Speculative {
            path: now.path.clone(),
            name: name.to_owned(),
            span: declared.span,
        })
        .collect()
}

fn referenced(name: &str, declared_in: &Path, own: &[Span], all: &[Source]) -> bool {
    all.iter().any(|source| {
        let outside = |at: &Span| {
            source.path != declared_in || !own.iter().any(|declaration| within(*at, *declaration))
        };
        let mentioned = |spans: &BTreeSet<Span>| spans.iter().any(outside);

        source.facts.names.get(name).is_some_and(mentioned)
            || source.facts.mentions.get(name).is_some_and(mentioned)
    })
}

pub fn mentioned_in(text: &str, name: &str) -> bool {
    text.match_indices(name).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + name.len()..].chars().next();
        !before.is_some_and(is_identifier_character) && !after.is_some_and(is_identifier_character)
    })
}

fn is_identifier_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn within(at: Span, declaration: Span) -> bool {
    declaration.start_line <= at.start_line && at.end_line <= declaration.end_line
}
