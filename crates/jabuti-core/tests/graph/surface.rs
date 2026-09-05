use std::path::{Path, PathBuf};

use jabuti_core::graph::facts::{self, Declared, FileFacts};
use jabuti_core::graph::index::{Index, Source};
use jabuti_core::graph::surface::{self, Roots, Speculative};
use jabuti_core::model::Span;
use jabuti_core::{lang, syntax};

use super::common::sources_under;

fn source<'a>(sources: &'a [Source], path: &str) -> &'a Source {
    sources
        .iter()
        .find(|source| source.path == Path::new(path))
        .unwrap_or_else(|| panic!("no source at {path}"))
}

fn declared<'a>(sources: &'a [Source], path: &str, name: &str) -> &'a Declared {
    &source(sources, path).facts.declares[name][0]
}

fn described(found: &[Speculative]) -> Vec<String> {
    found
        .iter()
        .map(|item| {
            format!(
                "{}:{} {}",
                item.path.display(),
                item.span.start_line,
                item.name
            )
        })
        .collect()
}

fn all_new(sources: &[Source], roots: &Roots) -> Vec<String> {
    let mut found = Vec::new();
    for source in sources {
        found.extend(described(&surface::speculative(
            source, None, sources, roots,
        )));
    }
    found
}

fn one_file(path: &str, text: &str) -> Vec<Source> {
    let parsed = syntax::parse(text, &lang::RUST).expect("parses");

    vec![Source {
        path: PathBuf::from(path),
        language: lang::LanguageId::Rust,
        facts: facts::facts(&parsed),
    }]
}

#[test]
fn a_declaration_records_its_span_visibility_owner_and_whether_an_attribute_marks_it() {
    let sources = sources_under("surface/lib", &lang::RUST);

    assert_eq!(
        declared(&sources, "src/internal.rs", "Hidden"),
        &Declared {
            span: Span {
                start_line: 7,
                end_line: 7
            },
            public: true,
            marked: false,
            owner: None,
        }
    );
    assert_eq!(
        declared(&sources, "src/internal.rs", "open")
            .owner
            .as_deref(),
        Some("Exposed")
    );
    assert!(
        !declared(&sources, "src/internal.rs", "Plain").marked,
        "derive is inert"
    );
    assert!(
        declared(&sources, "src/internal.rs", "entry").marked,
        "no_mangle makes a root we cannot follow"
    );
    assert!(
        !declared(&sources, "src/api.rs", "plumbing").public,
        "pub(crate) is not the public surface"
    );
}

#[test]
fn an_attribute_still_marks_its_item_when_a_comment_sits_between_them() {
    let sources = sources_under("surface/bin", &lang::RUST);

    assert!(declared(&sources, "src/tasks.rs", "entry").marked);
}

#[test]
fn an_item_inside_a_test_module_is_marked_as_test_code() {
    let sources = sources_under("surface/bin", &lang::RUST);

    assert!(declared(&sources, "src/tasks.rs", "helper").marked);
}

#[test]
fn a_kotlin_declaration_is_public_unless_a_modifier_restricts_it_and_marked_by_any_annotation() {
    let sources = sources_under("surface/kotlin", &lang::KOTLIN);
    let catalog = &source(&sources, "Catalog.kt").facts;

    assert!(catalog.declares["Basket"][0].public);
    assert!(!catalog.declares["Basket"][0].marked);
    assert!(!catalog.declares["Ledger"][0].public);
    assert!(catalog.declares["CatalogController"][0].marked);
}

#[test]
fn the_crate_root_exports_its_public_modules_re_exports_by_name_and_follows_globs_to_a_file() {
    let sources = sources_under("surface/lib", &lang::RUST);
    let roots = surface::roots(&sources, &Index::of(&sources));

    assert_eq!(
        roots,
        Roots {
            exported: [
                "src/api.rs",
                "src/extra/items.rs",
                "src/job.rs",
                "src/lib.rs",
            ]
            .into_iter()
            .map(PathBuf::from)
            .collect(),
            surface: [
                "Bolt", "Deep", "Exposed", "Gadget", "Job", "api", "plumbing", "schedule", "serve",
                "spare", "widget",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        }
    );
}

#[test]
fn a_binary_crate_exports_nothing_so_only_main_is_a_root() {
    let sources = sources_under("surface/bin", &lang::RUST);

    assert_eq!(
        surface::roots(&sources, &Index::of(&sources)),
        Roots::default()
    );
}

#[test]
fn in_a_library_only_a_public_item_the_crate_does_not_export_and_nothing_uses_is_speculative() {
    let sources = sources_under("surface/lib", &lang::RUST);
    let roots = surface::roots(&sources, &Index::of(&sources));

    assert_eq!(
        all_new(&sources, &roots),
        ["src/internal.rs:14 Plain"],
        "Hidden is named by a pub(crate) use, open belongs to the re-exported Exposed"
    );
}

#[test]
fn in_a_binary_every_public_item_nothing_uses_is_speculative_and_a_value_use_counts() {
    let sources = sources_under("surface/bin", &lang::RUST);
    let roots = surface::roots(&sources, &Index::of(&sources));

    assert_eq!(
        all_new(&sources, &roots),
        [
            "src/tasks.rs:17 pause",
            "src/tasks.rs:22 orphan",
            "src/tasks.rs:41 make"
        ],
        "MAX and orphan_handler are used as values, B::make is public and unused, A::make is private"
    );
}

#[test]
fn a_kotlin_declaration_used_as_a_value_or_imported_elsewhere_is_referenced() {
    let sources = sources_under("surface/kotlin", &lang::KOTLIN);
    let roots = surface::roots(&sources, &Index::of(&sources));

    assert_eq!(
        all_new(&sources, &roots),
        ["Catalog.kt:12 Basket"],
        "Crate is imported by another package, Registry is passed as a value"
    );
}

#[test]
fn a_declaration_the_earlier_revision_already_had_is_not_new() {
    let sources = sources_under("surface/bin", &lang::RUST);
    let roots = surface::roots(&sources, &Index::of(&sources));
    let tasks = source(&sources, "src/tasks.rs");
    let mut then = FileFacts::default();
    then.declares
        .insert("orphan".to_owned(), tasks.facts.declares["orphan"].clone());

    assert_eq!(
        described(&surface::speculative(tasks, Some(&then), &sources, &roots)),
        ["src/tasks.rs:17 pause", "src/tasks.rs:41 make"]
    );
}

#[test]
fn a_reference_inside_the_declaration_itself_does_not_count() {
    let sources = one_file("src/main.rs", "pub fn spin() {\n    spin();\n}\n");

    assert_eq!(
        described(&surface::speculative(
            &sources[0],
            None,
            &sources,
            &Roots::default()
        )),
        ["src/main.rs:1 spin"]
    );
}

#[test]
fn a_recursive_call_inside_a_same_named_private_item_does_not_keep_the_public_one_alive() {
    let sources = one_file(
        "src/main.rs",
        "pub struct A;\n\nimpl A {\n    pub fn make() -> A {\n        A\n    }\n}\n\npub struct B;\n\nimpl B {\n    fn make(n: u32) -> u32 {\n        if n == 0 { 0 } else { B::make(n - 1) }\n    }\n}\n",
    );

    assert_eq!(
        described(&surface::speculative(
            &sources[0],
            None,
            &sources,
            &Roots::default()
        )),
        ["src/main.rs:4 make"],
        "A and B are named by their own impl blocks, the private make keeps only itself alive"
    );
}

#[test]
fn a_name_counts_as_mentioned_only_as_a_whole_word() {
    assert!(surface::mentioned_in("val shelf = Shelf()", "Shelf"));
    assert!(surface::mentioned_in("Shelf.arrange()", "Shelf"));
    assert!(!surface::mentioned_in(
        "val bookshelf = Bookshelf()",
        "Shelf"
    ));
    assert!(!surface::mentioned_in("Shelf_2()", "Shelf"));
    assert!(!surface::mentioned_in("", "Shelf"));
}
