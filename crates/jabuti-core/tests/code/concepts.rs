use std::collections::BTreeMap;

use jabuti_core::code::concepts;
use jabuti_core::graph::facts;
use jabuti_core::lang::{self, LangSpec, LanguageId};
use jabuti_core::model::{Concept, ConceptBindings};
use jabuti_core::syntax;
use rstest::rstest;

use super::common::read_fixture;

fn rendered(relative: &str, spec: &'static LangSpec) -> Vec<String> {
    let source = read_fixture(relative);
    let parsed = syntax::parse(&source, spec).expect("the fixture parses cleanly");

    concepts::occurrences(&parsed, &ConceptBindings::default(), &BTreeMap::new())
        .into_iter()
        .map(|occurrence| {
            format!(
                "{} {} {}..{}",
                occurrence.concept.id(),
                occurrence.subject,
                occurrence.span.start_line,
                occurrence.span.end_line
            )
        })
        .collect()
}

#[test]
fn rust_bindings_map_each_error_masking_shape_and_leave_tests_out() {
    assert_eq!(
        rendered("concepts/rust.rs", &lang::RUST),
        [
            "error-panic unwrap 2..2",
            "error-panic expect 3..3",
            "error-discard _ 4..4",
            "error-discard ok 5..5",
            "error-swallow Err 6..6",
        ]
    );
}

#[test]
fn kotlin_bindings_map_each_error_masking_shape_and_leave_tests_out() {
    assert_eq!(
        rendered("concepts/kotlin.kt", &lang::KOTLIN),
        [
            "error-panic !! 2..2",
            "error-discard getOrNull 3..3",
            "error-swallow catch 6..6",
        ]
    );
}

#[test]
fn typescript_bindings_map_empty_handlers_but_not_recovery() {
    assert_eq!(
        rendered("concepts/typescript.ts", &lang::TYPESCRIPT),
        ["error-swallow catch 4..4", "error-swallow catch 7..7"]
    );
}

#[test]
fn a_user_binding_adds_a_project_api_without_replacing_built_ins() {
    let source = "fn live() {\n    mycorp::discard();\n    read().unwrap();\n}\n";
    let parsed = syntax::parse(source, &lang::RUST).expect("source parses cleanly");
    let mut bindings = ConceptBindings::default();
    bindings.set(
        LanguageId::Rust,
        Concept::ErrorDiscard,
        vec!["mycorp::discard".to_owned()],
    );

    let found = concepts::occurrences(&parsed, &bindings, &BTreeMap::new());

    assert_eq!(
        found
            .iter()
            .map(|occurrence| (occurrence.concept, occurrence.subject.as_str()))
            .collect::<Vec<_>>(),
        [
            (Concept::ErrorDiscard, "discard"),
            (Concept::ErrorPanic, "unwrap"),
        ]
    );
}

#[rstest]
#[case("#[tokio::test(flavor = \"multi_thread\")]")]
#[case("#[test_log::test(tokio::test)]")]
#[case("#[rstest(value, case(1))]")]
fn a_rust_test_attribute_with_arguments_suppresses_its_concepts(#[case] attribute: &str) {
    let source = format!("{attribute}\nasync fn checks() {{\n    read().unwrap();\n}}\n");
    let parsed = syntax::parse(&source, &lang::RUST).expect("source parses cleanly");

    assert_eq!(
        concepts::occurrences(&parsed, &ConceptBindings::default(), &BTreeMap::new()),
        []
    );
}

#[test]
fn concept_ids_round_trip_without_colliding() {
    let resolved: Vec<Concept> = Concept::ALL
        .into_iter()
        .map(|concept| Concept::from_id(concept.id()).expect("published concept resolves"))
        .collect();

    assert_eq!(resolved, Concept::ALL);
    assert_eq!(Concept::from_id("unknown"), None);
}

#[rstest]
#[case(
    &lang::RUST,
    "use mycorp::errors::discard as ignore;\nfn live() { ignore(); }\n",
    "mycorp::errors::discard",
    "ignore"
)]
#[case(
    &lang::KOTLIN,
    "import mycorp.errors.discard as ignore\nfun live() { ignore() }\n",
    "mycorp.errors.discard",
    "ignore"
)]
#[case(
    &lang::TYPESCRIPT,
    "import { discard as ignore } from \"@mycorp/errors\";\nexport function live(): void { ignore(); }\n",
    "@mycorp/errors.discard",
    "ignore"
)]
#[case(
    &lang::TYPESCRIPT,
    "import * as errors from \"@mycorp/errors\";\nexport function live(): void { errors.discard(); }\n",
    "@mycorp/errors.discard",
    "discard"
)]
fn imported_aliases_retain_their_concept_binding(
    #[case] spec: &'static LangSpec,
    #[case] source: &str,
    #[case] canonical: &str,
    #[case] expected_subject: &str,
) {
    let parsed = syntax::parse(source, spec).expect("source parses cleanly");
    let aliases = facts::aliases(&parsed);
    let mut bindings = ConceptBindings::default();
    bindings.set(spec.id, Concept::ErrorDiscard, vec![canonical.to_owned()]);

    assert_eq!(
        concepts::occurrences(&parsed, &bindings, &aliases),
        [concepts::Occurrence {
            concept: Concept::ErrorDiscard,
            subject: expected_subject.to_owned(),
            span: jabuti_core::model::Span {
                start_line: 2,
                end_line: 2,
            },
        }]
    );
}
