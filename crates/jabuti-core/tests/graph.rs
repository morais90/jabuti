mod common;

use jabuti_core::model::FileFacts;
use jabuti_core::{graph, lang, syntax};

fn facts_of(relative: &str, spec: &'static lang::LangSpec) -> FileFacts {
    let source = common::read_fixture(relative);
    syntax::parse(&source, spec)
        .expect("the fixture parses cleanly")
        .facts()
}

fn rendered(facts: &FileFacts) -> String {
    let mut lines = vec![format!("module {}", facts.module)];
    lines.extend(facts.declares.iter().map(|name| format!("declares {name}")));
    lines.extend(facts.paths.iter().map(|path| format!("path {path}")));
    lines.extend(facts.names.iter().map(|name| format!("name {name}")));

    lines.join("\n")
}

#[test]
fn a_rust_file_reports_every_path_it_writes_wherever_it_wrote_it() {
    let facts = facts_of("rust/references.rs", &lang::RUST);

    assert_eq!(
        rendered(&facts),
        "module \n\
         path crate::config::Settings\n\
         path crate::git::run\n\
         path crate::policy\n\
         path crate::policy::Policy\n\
         path crate::policy::Rule\n\
         path crate::policy::defaults::strict\n\
         path crate::report::render::agent::Line\n\
         path crate::tools::probe::name\n\
         path self::inner::Helper\n\
         path serde::Serialize\n\
         path std::collections::BTreeMap\n\
         path super::git\n\
         path super::scan\n\
         path super::since::Changes::new\n\
         path super::tools::probe"
    );
}

#[test]
fn a_path_into_another_package_survives_extraction_and_dies_at_resolution() {
    let sources = sources_under("graph/rust", &lang::RUST);
    let external = sources.iter().any(|source| {
        source
            .facts
            .paths
            .iter()
            .any(|path| path.starts_with("std::"))
    });

    assert!(
        external,
        "the fixture should write at least one external path"
    );

    let edges = graph::edges(&sources);

    assert!(
        edges.iter().all(|(_, to)| to.starts_with("src/")),
        "{}",
        drawn(&edges)
    );
}

#[test]
fn a_path_written_inside_a_macro_is_still_a_reference() {
    let facts = facts_of("rust/references.rs", &lang::RUST);

    assert!(
        facts.paths.contains("crate::tools::probe::name"),
        "{:?}",
        facts.paths
    );
}

#[test]
fn a_reference_needs_a_separator_so_a_self_receiver_is_not_one() {
    let facts = facts_of("rust/references.rs", &lang::RUST);

    assert!(
        !facts.paths.iter().any(|path| path == "self"),
        "{:?}",
        facts.paths
    );
}

#[test]
fn a_kotlin_file_reports_its_package_its_declarations_and_every_bare_name() {
    let facts = facts_of("kotlin/references.kt", &lang::KOTLIN);

    insta::assert_snapshot!(rendered(&facts));
}

fn sources_under(relative: &str, spec: &'static lang::LangSpec) -> Vec<graph::Source> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative);

    let mut paths = Vec::new();
    gather(&root, &mut paths);
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            let source = std::fs::read_to_string(&path).expect("fixture readable");
            let facts = syntax::parse(&source, spec)
                .unwrap_or_else(|_| panic!("fixture {} parses cleanly", path.display()))
                .facts();
            let relative = path
                .strip_prefix(&root)
                .expect("under the root")
                .to_path_buf();

            graph::Source {
                path: relative,
                language: spec.id,
                facts,
            }
        })
        .collect()
}

fn gather(root: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
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

fn drawn(edges: &graph::Edges) -> String {
    edges
        .iter()
        .map(|(from, to)| format!("{} -> {}", from.display(), to.display()))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_rust_path_resolves_to_a_file_whether_or_not_it_came_from_a_use() {
    let edges = graph::edges(&sources_under("graph/rust", &lang::RUST));

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
    let edges = graph::edges(&sources_under("graph/kotlin", &lang::KOTLIN));

    assert_eq!(
        drawn(&edges),
        "catalog/Shelf.kt -> catalog/Book.kt\n\
         storage/Repository.kt -> catalog/Shelf.kt"
    );
}

#[test]
fn a_reference_into_another_crate_of_the_workspace_resolves() {
    let edges = graph::edges(&sources_under("graph/workspace", &lang::RUST));

    assert_eq!(
        drawn(&edges),
        "crates/app/src/main.rs -> crates/app/src/runner.rs\n\
         crates/app/src/main.rs -> crates/engine/src/lib.rs\n\
         crates/app/src/main.rs -> crates/engine/src/model.rs\n\
         crates/app/src/runner.rs -> crates/engine/src/lib.rs\n\
         crates/app/src/runner.rs -> crates/engine/src/model.rs"
    );
}
