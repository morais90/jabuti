use std::path::{Path, PathBuf};

use jabuti_core::graph::facts::{self, FileFacts};
use jabuti_core::graph::index::{self, Edges, Source};
use jabuti_core::graph::layers::{Layers, Violation, violations};
use jabuti_core::{lang, syntax};

fn read_fixture(relative: &str) -> String {
    let path = fixture_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing fixture {relative}"))
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph")
}

fn facts_of(relative: &str, spec: &'static lang::LangSpec) -> FileFacts {
    let source = read_fixture(relative);
    facts::facts(&syntax::parse(&source, spec).expect("the fixture parses cleanly"))
}

fn rendered(facts: &FileFacts) -> String {
    let mut lines = vec![format!("module {}", facts.module)];
    lines.extend(facts.declares.iter().map(|name| format!("declares {name}")));
    lines.extend(
        facts
            .paths
            .iter()
            .map(|(path, at)| format!("path {path} at {}", at.start_line)),
    );
    lines.extend(
        facts
            .names
            .iter()
            .map(|(name, at)| format!("name {name} at {}", at.start_line)),
    );

    lines.join("\n")
}

#[test]
fn a_rust_file_reports_every_path_it_writes_wherever_it_wrote_it() {
    let facts = facts_of("references.rs", &lang::RUST);

    assert_eq!(
        rendered(&facts),
        "module \n\
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
         path super::tools::probe at 5"
    );
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

fn sources_under(relative: &str, spec: &'static lang::LangSpec) -> Vec<Source> {
    let root = fixture_root().join(relative);

    let mut paths = Vec::new();
    gather(&root, &mut paths);
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            let source = std::fs::read_to_string(&path).expect("fixture readable");
            let facts = facts::facts(
                &syntax::parse(&source, spec)
                    .unwrap_or_else(|_| panic!("fixture {} parses cleanly", path.display())),
            );
            let relative = path
                .strip_prefix(&root)
                .expect("under the root")
                .to_path_buf();

            Source {
                path: relative,
                language: spec.id,
                facts,
            }
        })
        .collect()
}

fn gather(root: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(root)
        .expect("fixture directory")
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            gather(&path, found);
        } else {
            found.push(path);
        }
    }
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
