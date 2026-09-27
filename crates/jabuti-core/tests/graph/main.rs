use std::path::PathBuf;

mod common;
mod findings;
mod surface;

use common::{fixture_root, sources_under};
use jabuti_core::graph::facts::{self, FileFacts};
use jabuti_core::graph::index::{self, Edges};
use jabuti_core::graph::layers::{Violation, violations};
use jabuti_core::policy::Layers;
use jabuti_core::{lang, syntax};

fn read_fixture(relative: &str) -> String {
    let path = fixture_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing fixture {relative}"))
}

fn facts_of(relative: &str, spec: &'static lang::LangSpec) -> FileFacts {
    let source = read_fixture(relative);
    facts::facts(&syntax::parse(&source, spec).expect("the fixture parses cleanly"))
}

fn rendered(facts: &FileFacts) -> String {
    let mut lines = vec![format!("module {}", facts.module)];
    lines.extend(facts.declares.iter().flat_map(|(name, declarations)| {
        declarations.iter().map(move |declared| {
            format!(
                "declares {name} at {} {}{}{}",
                declared.span.start_line,
                if declared.public { "public" } else { "private" },
                if declared.marked { " marked" } else { "" },
                declared
                    .owner
                    .as_ref()
                    .map(|owner| format!(" on {owner}"))
                    .unwrap_or_default()
            )
        })
    }));
    lines.extend(facts.exports.iter().map(|name| format!("exports {name}")));
    lines.extend(
        facts
            .glob_exports
            .iter()
            .map(|path| format!("exports {path}::*")),
    );
    lines.extend(
        facts
            .paths
            .iter()
            .map(|(path, at)| format!("path {path} at {}", at.start_line)),
    );
    lines.extend(facts.names.iter().map(|(name, spans)| {
        let lines: Vec<String> = spans.iter().map(|at| at.start_line.to_string()).collect();
        format!("name {name} at {}", lines.join(","))
    }));
    lines.extend(facts.mentions.iter().map(|(name, spans)| {
        let lines: Vec<String> = spans.iter().map(|at| at.start_line.to_string()).collect();
        format!("mention {name} at {}", lines.join(","))
    }));

    lines.join("\n")
}

const RUST_FACTS: &str = "module \n\
         declares Widget at 13 public\n\
         declares describe at 27 public on Widget\n\
         declares draw at 19 public on Widget\n\
         path crate::config::Settings at 1\n\
         path crate::git::run at 20\n\
         path crate::policy::Policy at 3\n\
         path crate::policy::Rule at 3\n\
         path crate::policy::defaults::strict at 21\n\
         path crate::render::agent::Line at 6\n\
         path crate::render::agent::Width at 6\n\
         path crate::render::theme at 6\n\
         path crate::report::render::agent::Line at 2\n\
         path crate::tools::probe::name at 28\n\
         path self::inner::Helper at 7\n\
         path serde::Serialize at 9\n\
         path std::collections::BTreeMap at 8\n\
         path super::git at 4\n\
         path super::scan at 5\n\
         path super::since::Changes::new at 22\n\
         path super::tools::probe at 5\n\
         mention BTreeMap at 8,15\n\
         mention Changes at 22\n\
         mention Helper at 7\n\
         mention Line at 2,6,15\n\
         mention Named at 3\n\
         mention Policy at 3\n\
         mention Row at 6\n\
         mention Rule at 3\n\
         mention Serialize at 9\n\
         mention Settings at 1,14\n\
         mention String at 27\n\
         mention Width at 6\n\
         mention agent at 2,6\n\
         mention collections at 8\n\
         mention config at 1\n\
         mention defaults at 21\n\
         mention format at 28\n\
         mention git at 4,20\n\
         mention head at 20,24\n\
         mention helper at 22,24\n\
         mention inner at 7,11\n\
         mention len at 24\n\
         mention lines at 15,24\n\
         mention name at 28\n\
         mention new at 22\n\
         mention policy at 3,21,24\n\
         mention probe at 5,28\n\
         mention render at 2,6\n\
         mention report at 2\n\
         mention run at 20\n\
         mention scan at 5\n\
         mention serde at 9\n\
         mention settings at 14\n\
         mention since at 22\n\
         mention std at 8\n\
         mention strict at 21\n\
         mention theme at 6\n\
         mention tools at 5,28";

#[test]
fn a_rust_file_reports_every_path_it_writes_wherever_it_wrote_it() {
    let facts = facts_of("references.rs", &lang::RUST);

    assert_eq!(rendered(&facts), RUST_FACTS);
}

#[test]
fn a_path_into_another_package_survives_extraction_and_dies_at_resolution() {
    let sources = sources_under("rust", &lang::RUST);
    let external = sources.iter().any(|source| {
        source
            .facts
            .paths
            .iter()
            .any(|(path, _)| path.starts_with("std::"))
    });

    assert!(
        external,
        "the fixture should write at least one external path"
    );

    let edges = index::edges(&sources);

    assert!(
        edges.keys().all(|(_, to)| to.starts_with("src/")),
        "{}",
        drawn(&edges)
    );
}

#[test]
fn a_path_written_inside_a_macro_is_still_a_reference() {
    let facts = facts_of("references.rs", &lang::RUST);

    assert!(
        facts.paths.contains_key("crate::tools::probe::name"),
        "{:?}",
        facts.paths
    );
}

#[test]
fn a_reference_needs_a_separator_so_a_self_receiver_is_not_one() {
    let facts = facts_of("references.rs", &lang::RUST);

    assert!(
        !facts.paths.keys().any(|path| path == "self"),
        "{:?}",
        facts.paths
    );
}

#[test]
fn a_kotlin_file_reports_its_package_its_declarations_and_every_bare_name() {
    let facts = facts_of("references.kt", &lang::KOTLIN);

    insta::assert_snapshot!(rendered(&facts));
}

#[test]
fn a_typescript_file_reports_exports_declarations_and_relative_modules() {
    let facts = facts_of("typescript/catalog/Shelf.ts", &lang::TYPESCRIPT);

    insta::assert_snapshot!(rendered(&facts));
}

fn drawn(edges: &Edges) -> String {
    edges
        .keys()
        .map(|(from, to)| format!("{} -> {}", from.display(), to.display()))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_rust_path_resolves_to_a_file_whether_or_not_it_came_from_a_use() {
    let edges = index::edges(&sources_under("rust", &lang::RUST));

    assert_eq!(
        drawn(&edges),
        "src/config.rs -> src/git.rs\n\
         src/main.rs -> src/config.rs\n\
         src/main.rs -> src/git.rs\n\
         src/main.rs -> src/report/agent.rs\n\
         src/report/agent.rs -> src/config.rs\n\
         src/report/agent.rs -> src/git.rs\n\
         src/report/agent.rs -> src/report/theme.rs\n\
         src/report/mod.rs -> src/config.rs\n\
         src/report/mod.rs -> src/report/agent.rs\n\
         src/report/mod.rs -> src/report/theme.rs"
    );
}

#[test]
fn a_kotlin_file_depends_on_a_sibling_it_never_imported() {
    let edges = index::edges(&sources_under("kotlin", &lang::KOTLIN));

    assert_eq!(
        drawn(&edges),
        "catalog/Shelf.kt -> catalog/Book.kt\n\
         storage/Repository.kt -> catalog/Shelf.kt"
    );
}

#[test]
fn a_typescript_file_resolves_extensionless_javascript_and_declaration_modules() {
    let edges = index::edges(&sources_under("typescript", &lang::TYPESCRIPT));

    assert_eq!(
        drawn(&edges),
        "catalog/Shelf.ts -> catalog/Book.ts\n\
         storage/DetailsConsumer.ts -> catalog/details/index.ts\n\
         storage/Repository.ts -> catalog/Shelf.ts\n\
         storage/TypesConsumer.ts -> catalog/Types.d.ts"
    );
}

#[test]
fn a_reference_into_another_crate_or_into_the_crate_root_resolves() {
    let edges = index::edges(&sources_under("workspace", &lang::RUST));

    assert_eq!(
        drawn(&edges),
        "crates/app/src/main.rs -> crates/app/src/runner.rs\n\
         crates/app/src/main.rs -> crates/engine/src/lib.rs\n\
         crates/app/src/main.rs -> crates/engine/src/model.rs\n\
         crates/app/src/runner.rs -> crates/engine/src/lib.rs\n\
         crates/app/src/runner.rs -> crates/engine/src/model.rs\n\
         crates/engine/src/model.rs -> crates/engine/src/lib.rs"
    );
}

fn layered(assignments: &[(&str, &str)], allowed: &[(&str, &[&str])]) -> Layers {
    Layers {
        of: assignments
            .iter()
            .map(|(path, layer)| (PathBuf::from(path), (*layer).to_owned()))
            .collect(),
        allowed: allowed
            .iter()
            .map(|(layer, targets)| {
                (
                    (*layer).to_owned(),
                    targets.iter().map(|target| (*target).to_owned()).collect(),
                )
            })
            .collect(),
    }
}

fn described(violations: &[Violation]) -> String {
    violations
        .iter()
        .map(|violation| {
            format!(
                "{}:{} {} -> {} ({} may not depend on {})",
                violation.from.display(),
                violation.at.start_line,
                violation.from_layer,
                violation.to_layer,
                violation.from_layer,
                violation.to_layer
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn every_reference_into_a_layer_that_was_not_allowed_is_a_violation_at_its_own_line() {
    let edges = index::edges(&sources_under("workspace", &lang::RUST));
    let layers = layered(
        &[
            ("crates/app/src/main.rs", "app"),
            ("crates/app/src/runner.rs", "app"),
            ("crates/engine/src/lib.rs", "engine"),
            ("crates/engine/src/model.rs", "engine"),
        ],
        &[("app", &[]), ("engine", &[])],
    );

    assert_eq!(
        described(&violations(&edges, &layers)),
        "crates/app/src/main.rs:7 app -> engine (app may not depend on engine)\n\
         crates/app/src/main.rs:1 app -> engine (app may not depend on engine)\n\
         crates/app/src/runner.rs:5 app -> engine (app may not depend on engine)\n\
         crates/app/src/runner.rs:1 app -> engine (app may not depend on engine)"
    );
}

#[test]
fn a_dependency_a_layer_was_allowed_to_have_is_not_reported() {
    let edges = index::edges(&sources_under("workspace", &lang::RUST));
    let layers = layered(
        &[
            ("crates/app/src/main.rs", "app"),
            ("crates/app/src/runner.rs", "app"),
            ("crates/engine/src/lib.rs", "engine"),
            ("crates/engine/src/model.rs", "engine"),
        ],
        &[("app", &["engine"]), ("engine", &[])],
    );

    assert!(violations(&edges, &layers).is_empty());
}

#[test]
fn a_file_in_no_layer_neither_violates_nor_is_violated() {
    let edges = index::edges(&sources_under("workspace", &lang::RUST));
    let layers = layered(&[("crates/app/src/main.rs", "app")], &[("app", &[])]);

    assert!(violations(&edges, &layers).is_empty());
}

#[test]
fn a_dependency_inside_one_layer_is_never_a_violation() {
    let edges = index::edges(&sources_under("workspace", &lang::RUST));
    let layers = layered(
        &[
            ("crates/app/src/main.rs", "everything"),
            ("crates/app/src/runner.rs", "everything"),
            ("crates/engine/src/lib.rs", "everything"),
            ("crates/engine/src/model.rs", "everything"),
        ],
        &[("everything", &[])],
    );

    assert!(violations(&edges, &layers).is_empty());
}

#[test]
fn an_edge_reached_by_several_references_sits_on_the_earliest_of_them() {
    let edges = index::edges(&sources_under("rust", &lang::RUST));
    let at = edges
        .get(&(PathBuf::from("src/config.rs"), PathBuf::from("src/git.rs")))
        .expect("config depends on git");

    assert_eq!(at.start_line, 1);
}
