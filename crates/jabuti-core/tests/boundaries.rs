use std::path::{Path, PathBuf};

const KERNEL: [&str; 5] = ["lang", "model", "policy", "report", "syntax"];
const CONTEXTS: [&str; 4] = ["code", "graph", "history", "tools"];

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
    for context in CONTEXTS {
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
