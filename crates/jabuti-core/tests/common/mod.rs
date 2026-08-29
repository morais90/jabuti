#![allow(dead_code)]

use std::path::PathBuf;

use jabuti_core::lang;
use jabuti_core::metrics::LineIndex;
use jabuti_core::model::{Unit, UnitKind};
use jabuti_core::syntax::{self, Parsed, SyntaxError};

pub fn read_fixture(relative: &str) -> String {
    let path = fixture_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing fixture {relative}"))
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub fn parse_fixture(source: &str) -> Parsed<'_> {
    syntax::parse(source, &lang::RUST).expect("fixture parses cleanly")
}

pub fn units_of(relative: &str) -> Unit {
    parse_fixture(&read_fixture(relative)).units()
}

pub fn line_index_of(relative: &str) -> LineIndex {
    let source = read_fixture(relative);
    let parsed = parse_fixture(&source);

    LineIndex::new(&source, &parsed.comment_ranges())
}

pub fn parse_outcome(relative: &str) -> Result<(), SyntaxError> {
    syntax::parse(&read_fixture(relative), &lang::RUST).map(|_| ())
}

pub fn find_unit<'a>(unit: &'a Unit, name: &str) -> &'a Unit {
    descend(unit, name).unwrap_or_else(|| panic!("no unit named {name}"))
}

fn descend<'a>(unit: &'a Unit, name: &str) -> Option<&'a Unit> {
    if unit.name.as_deref() == Some(name) {
        return Some(unit);
    }
    unit.children.iter().find_map(|child| descend(child, name))
}

pub fn kinds(units: &[Unit]) -> Vec<UnitKind> {
    units.iter().map(|unit| unit.kind).collect()
}

pub fn outline(unit: &Unit) -> String {
    let mut rendered = String::new();
    write_outline(unit, 0, &mut rendered);
    rendered
}

fn write_outline(unit: &Unit, depth: usize, rendered: &mut String) {
    let indent = "  ".repeat(depth);
    let name = unit.name.as_deref().unwrap_or("-");
    let span = unit.span;

    rendered.push_str(&format!(
        "{indent}{:?} {name} {}..{}\n",
        unit.kind, span.start_line, span.end_line
    ));

    for child in &unit.children {
        write_outline(child, depth + 1, rendered);
    }
}
