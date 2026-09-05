mod common;

use common::parse_outcome;
use jabuti_core::lang;
use jabuti_core::syntax::{self, SyntaxError};

#[test]
fn every_registered_language_parses_an_empty_file() {
    for spec in lang::ALL {
        assert!(syntax::parse("", spec).is_ok(), "{:?}", spec.id);
    }
}

#[test]
fn source_that_does_not_parse_is_rejected_and_says_where() {
    let parsed = parse_outcome("syntax/malformed.rs");

    assert!(
        matches!(parsed, Err(SyntaxError::Malformed { line: 1 })),
        "{parsed:?}"
    );
}
