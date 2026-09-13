use std::collections::BTreeMap;

use jabuti_core::code::concepts;
use jabuti_core::graph::facts;
use jabuti_core::lang::{self, LangSpec, LanguageId};
use jabuti_core::model::{Concept, ConceptBindings};
use jabuti_core::syntax;
use rstest::rstest;

use super::common::read_fixture;

fn concept_pairs(
    parsed: &syntax::Parsed<'_>,
    bindings: &ConceptBindings,
) -> Vec<(Concept, String)> {
    concepts::occurrences(parsed, bindings, &BTreeMap::new())
        .into_iter()
        .map(|occurrence| (occurrence.concept, occurrence.subject))
        .collect()
}

fn rendered(relative: &str, spec: &'static LangSpec) -> Vec<String> {
    let source = read_fixture(relative);
    let parsed = syntax::parse(&source, spec).expect("the fixture parses cleanly");

    concepts::occurrences(&parsed, &ConceptBindings::default(), &BTreeMap::new())
        .into_iter()
        .map(|occurrence| {
            format!(
                "{} {} {}..{} {}",
                occurrence.concept.id(),
                occurrence.subject,
                occurrence.span.start_line,
                occurrence.span.end_line,
                if occurrence.in_test {
                    "test"
                } else {
                    "production"
                },
            )
        })
        .collect()
}

#[test]
fn rust_bindings_map_each_shape_and_tag_test_declarations() {
    assert_eq!(
        rendered("concepts/rust.rs", &lang::RUST),
        [
            "suppression (clippy::all) 1..1 production",
            "error-panic unwrap 4..4 production",
            "error-panic expect 5..5 production",
            "error-discard _ 6..6 production",
            "error-discard ok 7..7 production",
            "error-swallow Err 8..8 production",
            "error-panic unwrap 14..14 test",
            "suppression (dead_code) 17..17 production",
            "assertion assert 21..21 production",
        ]
    );
}

#[test]
fn kotlin_bindings_map_each_shape_and_tag_test_declarations() {
    assert_eq!(
        rendered("concepts/kotlin.kt", &lang::KOTLIN),
        [
            "error-panic !! 2..2 production",
            "error-discard getOrNull 3..3 production",
            "error-swallow catch 6..6 production",
            "error-panic !! 13..13 test",
            "assertion assertTrue 18..18 production",
            "suppression (\"UNCHECKED_CAST\") 21..21 production",
        ]
    );
}

#[test]
fn typescript_bindings_map_empty_handlers_but_not_recovery() {
    assert_eq!(
        rendered("concepts/typescript.ts", &lang::TYPESCRIPT),
        [
            "error-swallow catch 4..4 production",
            "error-swallow catch 7..7 production",
            "suppression // @ts-ignore 11..11 production",
            "suppression // eslint-disable-next-line no-console 14..14 production",
            "suppression any 18..18 production",
            "assertion expect 22..22 production",
        ]
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

    assert_eq!(
        concept_pairs(&parsed, &bindings),
        [
            (Concept::ErrorDiscard, "discard".to_owned()),
            (Concept::ErrorPanic, "unwrap".to_owned()),
        ]
    );
}

#[test]
fn call_sites_keep_only_bare_names_and_leave_method_and_qualified_calls_out() {
    let source = "fn live() {\n    helper();\n    self.helper();\n    Type::helper();\n    obj.helper();\n}\n";
    let parsed = syntax::parse(source, &lang::RUST).expect("source parses cleanly");

    let names: Vec<String> = concepts::call_sites(&parsed)
        .into_iter()
        .map(|call| call.name)
        .collect();

    assert_eq!(names, ["helper"]);
}

#[test]
fn rust_recognises_assert_cmds_fluent_assert_and_qualified_insta_macros() {
    let source = "fn checks() {\n    command().assert().success();\n    insta::assert_snapshot!(value);\n}\n";
    let parsed = syntax::parse(source, &lang::RUST).expect("source parses cleanly");

    assert_eq!(
        concept_pairs(&parsed, &ConceptBindings::default()),
        [
            (Concept::Assertion, "assert".to_owned()),
            (Concept::Assertion, "assert_snapshot".to_owned()),
        ]
    );
}

#[test]
fn typescript_ts_nocheck_and_the_bracket_any_cast_are_tagged_as_suppression() {
    let source =
        "// @ts-nocheck\nfunction live(value: unknown): unknown {\n    return <any>value;\n}\n";
    let parsed = syntax::parse(source, &lang::TYPESCRIPT).expect("source parses cleanly");

    assert_eq!(
        concept_pairs(&parsed, &ConceptBindings::default()),
        [
            (Concept::Suppression, "// @ts-nocheck".to_owned()),
            (Concept::Suppression, "any".to_owned()),
        ]
    );
}

#[rstest]
#[case("#[tokio::test(flavor = \"multi_thread\")]")]
#[case("#[test_log::test(tokio::test)]")]
#[case("#[rstest(value, case(1))]")]
fn a_rust_test_attribute_with_arguments_tags_its_concepts_as_test_code(#[case] attribute: &str) {
    let source = format!("{attribute}\nasync fn checks() {{\n    read().unwrap();\n}}\n");
    let parsed = syntax::parse(&source, &lang::RUST).expect("source parses cleanly");

    let found = concepts::occurrences(&parsed, &ConceptBindings::default(), &BTreeMap::new());

    assert_eq!(found.len(), 1);
    assert!(found[0].in_test);
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
            in_test: false,
        }]
    );
}
