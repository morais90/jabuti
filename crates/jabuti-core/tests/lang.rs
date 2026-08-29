use std::path::Path;

use jabuti_core::lang::{self, LanguageId};

#[test]
fn a_rust_file_is_detected_by_its_extension() {
    let spec = lang::detect(Path::new("crates/jabuti-core/src/syntax.rs")).expect("rust is known");

    assert_eq!(spec.id, LanguageId::Rust);
    assert_eq!(spec.extensions, ["rs"]);
}

#[test]
fn a_file_of_an_unsupported_language_is_not_detected() {
    assert!(lang::detect(Path::new("README.md")).is_none());
}

#[test]
fn a_file_without_an_extension_is_not_detected() {
    assert!(lang::detect(Path::new("justfile")).is_none());
}
