use std::path::{Path, PathBuf};

const KERNEL: [&str; 7] = [
    "catalog", "diff", "lang", "model", "policy", "report", "syntax",
];
const CONTEXTS: [&str; 5] = ["code", "crossings", "graph", "history", "tools"];
const CROSSINGS: &str = "crossings";

fn source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(directory)
        .expect("directory listed")
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            found.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    found.sort();
    found
}

fn crate_paths_in(path: &Path) -> Vec<String> {
    let source = std::fs::read_to_string(path).expect("source readable");
    source
        .match_indices("crate::")
        .map(|(start, _)| {
            source[start + "crate::".len()..]
                .chars()
                .take_while(|character| character.is_alphanumeric() || *character == '_')
                .collect()
        })
        .collect()
}

#[test]
fn a_context_reaches_only_the_kernel_and_itself() {
    for context in CONTEXTS.into_iter().filter(|context| *context != CROSSINGS) {
        for file in rust_files(&source_root().join(context)) {
            for module in crate_paths_in(&file) {
                assert!(
                    KERNEL.contains(&module.as_str()) || module == context,
                    "{} reaches crate::{module}, which is outside the {context} context and the kernel",
                    file.display()
                );
            }
        }
    }
}

#[test]
fn only_the_crossings_reach_more_than_one_context() {
    for file in rust_files(&source_root().join(CROSSINGS)) {
        for module in crate_paths_in(&file) {
            assert!(
                KERNEL.contains(&module.as_str()) || CONTEXTS.contains(&module.as_str()),
                "{} reaches crate::{module}, which is neither the kernel nor a context",
                file.display()
            );
        }
    }
}

#[test]
fn the_kernel_reaches_no_context() {
    for module in KERNEL {
        let file = source_root().join(format!("{module}.rs"));
        for reached in crate_paths_in(&file) {
            assert!(
                KERNEL.contains(&reached.as_str()),
                "{} reaches crate::{reached}, which is a context",
                file.display()
            );
        }
    }
}

#[test]
fn the_catalog_reaches_nothing_but_the_language_table() {
    let file = source_root().join("catalog.rs");

    for reached in crate_paths_in(&file) {
        assert_eq!(
            reached,
            "lang",
            "{} reaches crate::{reached}, but what jabuti knows cannot depend on what a run produces",
            file.display()
        );
    }
}

#[test]
fn the_core_never_touches_the_file_system_a_process_or_the_environment() {
    for file in rust_files(&source_root()) {
        let source = std::fs::read_to_string(&file).expect("source readable");
        for call in [
            "std::fs",
            "fs::",
            "std::process",
            "std::env",
            "canonicalize",
        ] {
            assert!(
                !source.contains(call),
                "{} calls {call}, but the core only computes on what it is handed",
                file.display()
            );
        }
    }
}

#[test]
fn only_the_language_tables_and_the_catalog_name_a_language() {
    for file in rust_files(&source_root()) {
        let name = file.file_name().and_then(|name| name.to_str());
        if matches!(name, Some("lang.rs" | "catalog.rs")) {
            continue;
        }
        let source = std::fs::read_to_string(&file).expect("source readable");
        for language in ["Kotlin", "Rust", "TypeScript"] {
            assert!(
                !source.contains(&format!("LanguageId::{language}")),
                "{} branches on {language} instead of reading its language table",
                file.display()
            );
        }
    }
}

#[test]
fn every_context_and_kernel_module_the_boundary_names_exists() {
    for context in CONTEXTS {
        assert!(
            source_root().join(context).join("mod.rs").is_file(),
            "{context}"
        );
    }
    for module in KERNEL {
        assert!(
            source_root().join(format!("{module}.rs")).is_file(),
            "{module}"
        );
    }
}
