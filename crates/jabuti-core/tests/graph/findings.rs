use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use jabuti_core::catalog::{Concept, Rule, RuleId, Severity};
use jabuti_core::graph::index::Source;
use jabuti_core::graph::{self, Scan, facts};
use jabuti_core::lang::{self, LanguageId};
use jabuti_core::model::{Detail, Finding, Span};
use jabuti_core::policy::{ConceptBindings, Layers, Policy, RuleConfig};
use jabuti_core::syntax;

const MAIN: &str = "mod config;\nmod git;\nmod report;\n\nfn main() {\n    report::render();\n}\n";
const CONFIG: &str = "pub fn load() -> u32 {\n    crate::report::inner()\n}\n";
const GIT: &str = "pub fn run() -> u32 {\n    2\n}\n";
const REPORT_BEFORE: &str = "pub fn render() -> u32 {\n    crate::config::load();\n    crate::report::inner()\n}\n\npub fn inner() -> u32 {\n    3\n}\n";
const REPORT_NOW: &str = "pub fn render() -> u32 {\n    crate::config::load();\n    crate::git::run();\n    crate::report::inner()\n}\n\npub fn inner() -> u32 {\n    3\n}\n";

const BIN_MAIN: &str = "mod tasks;\n\nfn main() {\n    tasks::used();\n}\n";
const TASKS: &str = "pub fn used() {}\n\npub fn spare() {}\n";

fn source(path: &str, text: &str) -> Source {
    let parsed = syntax::parse(text, &lang::RUST).expect("fixture parses cleanly");

    Source {
        path: PathBuf::from(path),
        language: LanguageId::Rust,
        facts: facts::facts(&parsed),
    }
}

fn reporting() -> Vec<Source> {
    vec![
        source("src/main.rs", MAIN),
        source("src/config.rs", CONFIG),
        source("src/git.rs", GIT),
        source("src/report.rs", REPORT_NOW),
    ]
}

fn requested(paths: &[&str]) -> BTreeSet<PathBuf> {
    paths.iter().map(PathBuf::from).collect()
}

fn switched_off(rule: Rule) -> Policy {
    let mut policy = Policy::default();
    policy.set(
        rule,
        RuleConfig {
            limit: 0,
            severity: Severity::Off,
        },
    );
    policy
}

fn at(rule: Rule, place: (&str, u32), subject: Option<&str>, message: &str) -> Finding {
    let (path, line) = place;

    Finding {
        rule: RuleId::Native(rule),
        severity: Severity::Warning,
        path: path.to_owned(),
        span: Span {
            start_line: line,
            end_line: line,
        },
        subject: subject.map(str::to_owned),
        detail: Detail::Message {
            message: message.to_owned(),
        },
    }
}

fn separated() -> Layers {
    Layers {
        of: BTreeMap::from([
            (PathBuf::from("src/report.rs"), "ui".to_owned()),
            (PathBuf::from("src/config.rs"), "infra".to_owned()),
            (PathBuf::from("src/git.rs"), "infra".to_owned()),
        ]),
        allowed: BTreeMap::from([
            ("ui".to_owned(), BTreeSet::new()),
            ("infra".to_owned(), BTreeSet::new()),
        ]),
    }
}

#[test]
fn only_a_dependency_the_base_did_not_have_on_another_file_is_new() {
    let sources = reporting();
    let base = BTreeMap::from([(
        PathBuf::from("src/report.rs"),
        Some(source("src/report.rs", REPORT_BEFORE)),
    )]);
    let policy = Policy::default();

    let found = graph::findings(&Scan {
        requested: &requested(&["src/report.rs"]),
        sources: &sources,
        opaque: &[],
        base: &base,
        policy: &policy,
        compared: true,
        layers: None,
    });

    assert_eq!(
        found,
        [at(
            Rule::NewDependency,
            ("src/report.rs", 3),
            None,
            "now depends on src/git.rs"
        )]
    );
}

#[test]
fn nothing_is_compared_without_an_earlier_revision_or_while_the_rule_is_off() {
    let sources = reporting();
    let base = BTreeMap::from([(
        PathBuf::from("src/report.rs"),
        Some(source("src/report.rs", REPORT_BEFORE)),
    )]);
    let scan = |policy: &Policy, compared: bool| {
        graph::findings(&Scan {
            requested: &requested(&["src/report.rs"]),
            sources: &sources,
            opaque: &[],
            base: &base,
            policy,
            compared,
            layers: None,
        })
    };

    assert_eq!(scan(&Policy::default(), false), []);
    assert_eq!(scan(&switched_off(Rule::NewDependency), true), []);
}

#[test]
fn a_layer_is_crossed_only_from_an_examined_file_toward_another_file() {
    let sources = reporting();
    let layers = separated();
    let policy = Policy::default();

    let found = graph::findings(&Scan {
        requested: &requested(&["src/report.rs"]),
        sources: &sources,
        opaque: &[],
        base: &BTreeMap::new(),
        policy: &policy,
        compared: false,
        layers: Some(&layers),
    });

    assert_eq!(
        found,
        [
            at(
                Rule::LayerViolation,
                ("src/report.rs", 2),
                None,
                "ui may not depend on infra (src/config.rs)"
            ),
            at(
                Rule::LayerViolation,
                ("src/report.rs", 3),
                None,
                "ui may not depend on infra (src/git.rs)"
            ),
        ]
    );
}

#[test]
fn no_layer_is_checked_without_declared_layers_or_while_the_rule_is_off() {
    let sources = reporting();
    let layers = separated();
    let scan = |policy: &Policy, layers: Option<&Layers>| {
        graph::findings(&Scan {
            requested: &requested(&["src/report.rs"]),
            sources: &sources,
            opaque: &[],
            base: &BTreeMap::new(),
            policy,
            compared: false,
            layers,
        })
    };

    assert_eq!(scan(&Policy::default(), None), []);
    assert_eq!(scan(&switched_off(Rule::LayerViolation), Some(&layers)), []);
}

fn speculative(requested_paths: &[&str], opaque: &[String], policy: &Policy) -> Vec<Finding> {
    let sources = vec![
        source("src/main.rs", BIN_MAIN),
        source("src/tasks.rs", TASKS),
        source("tests/tasks.rs", TASKS),
    ];

    graph::findings(&Scan {
        requested: &requested(requested_paths),
        sources: &sources,
        opaque,
        base: &BTreeMap::new(),
        policy,
        compared: true,
        layers: None,
    })
}

#[test]
fn a_public_item_nothing_references_is_speculative_in_an_examined_production_file() {
    let policy = Policy::default();

    assert_eq!(
        speculative(&["src/tasks.rs", "tests/tasks.rs"], &[], &policy),
        [at(
            Rule::SpeculativeApi,
            ("src/tasks.rs", 3),
            Some("spare"),
            "public, and nothing references it"
        )]
    );
}

#[test]
fn a_public_item_is_not_speculative_where_it_is_not_examined_or_a_file_that_would_not_parse_names_it()
 {
    let policy = Policy::default();
    let unreadable = ["let _ = spare();".to_owned()];

    assert_eq!(speculative(&[], &[], &policy), []);
    assert_eq!(speculative(&["src/tasks.rs"], &unreadable, &policy), []);
    assert_eq!(
        speculative(&["src/tasks.rs"], &[], &switched_off(Rule::SpeculativeApi)),
        []
    );
}

#[test]
fn a_file_that_did_not_parse_at_the_base_is_not_judged_against_it() {
    let sources = vec![
        source("src/main.rs", BIN_MAIN),
        source("src/tasks.rs", TASKS),
    ];
    let base = BTreeMap::from([(PathBuf::from("src/tasks.rs"), None)]);
    let policy = Policy::default();

    let found = graph::findings(&Scan {
        requested: &requested(&["src/tasks.rs"]),
        sources: &sources,
        opaque: &[],
        base: &base,
        policy: &policy,
        compared: true,
        layers: None,
    });

    assert_eq!(found, []);
}

#[test]
fn aliases_are_read_only_for_a_language_the_project_bound_a_concept_in() {
    let text = "use std::process::exit as quit;\n\nfn stop() {\n    quit(1);\n}\n";
    let parsed = syntax::parse(text, &lang::RUST).expect("fixture parses cleanly");
    let mut bound = ConceptBindings::default();
    bound.set(
        LanguageId::Rust,
        Concept::ErrorPanic,
        vec!["std::process::exit".to_owned()],
    );

    assert_eq!(
        graph::aliases(&parsed, LanguageId::Rust, &ConceptBindings::default()),
        BTreeMap::new()
    );
    assert_eq!(
        graph::aliases(&parsed, LanguageId::Rust, &bound),
        BTreeMap::from([("quit".to_owned(), "std::process::exit".to_owned())])
    );
}
