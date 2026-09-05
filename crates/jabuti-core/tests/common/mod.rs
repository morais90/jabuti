use std::path::PathBuf;

use jabuti_core::lang;
use jabuti_core::syntax::{self, SyntaxError};

pub(crate) fn read_fixture(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing fixture {relative}"))
}

pub(crate) fn parse_outcome(relative: &str) -> Result<(), SyntaxError> {
    syntax::parse(&read_fixture(relative), &lang::RUST).map(|_| ())
}
