#![allow(dead_code)]

use std::fmt::Write;
use std::path::PathBuf;

use jabuti_core::code::metrics::{self, LineIndex};
use jabuti_core::code::units::{self, Unit};
use jabuti_core::lang;
use jabuti_core::model::UnitKind;
use jabuti_core::syntax::{self, Parsed};

pub(crate) fn read_fixture(relative: &str) -> String {
    let path = fixture_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing fixture {relative}"))
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/code")
}

pub(crate) fn parse_fixture(source: &str) -> Parsed<'_> {
    syntax::parse(source, &lang::RUST).expect("fixture parses cleanly")
}

pub(crate) fn units_of(relative: &str) -> Unit {
    units::units(&parse_fixture(&read_fixture(relative)))
}

pub(crate) fn line_index_of(relative: &str) -> LineIndex {
    let source = read_fixture(relative);
    let parsed = parse_fixture(&source);

    LineIndex::new(&source, &metrics::comment_ranges(&parsed))
}

pub(crate) fn find_unit<'a>(unit: &'a Unit, name: &str) -> &'a Unit {
    descend(unit, name).unwrap_or_else(|| panic!("no unit named {name}"))
}

fn descend<'a>(unit: &'a Unit, name: &str) -> Option<&'a Unit> {
    if unit.name.as_deref() == Some(name) {
        return Some(unit);
    }
    unit.children.iter().find_map(|child| descend(child, name))
}

pub(crate) fn kinds(units: &[Unit]) -> Vec<UnitKind> {
    units.iter().map(|unit| unit.kind).collect()
}

pub(crate) fn outline(unit: &Unit) -> String {
    let mut rendered = String::new();
    write_outline(unit, 0, &mut rendered);
    rendered
}

fn write_outline(unit: &Unit, depth: usize, rendered: &mut String) {
    let indent = "  ".repeat(depth);
    let name = unit.name.as_deref().unwrap_or("-");
    let span = unit.span;

    writeln!(
        rendered,
        "{indent}{:?} {name} {}..{}",
        unit.kind, span.start_line, span.end_line
    )
    .expect("writing to a string never fails");

    for child in &unit.children {
        write_outline(child, depth + 1, rendered);
    }
}
