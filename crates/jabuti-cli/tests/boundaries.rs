use std::path::{Path, PathBuf};

const KERNEL: [&str; 4] = ["config", "git", "main", "project"];
const CORE_KERNEL: [&str; 5] = ["lang", "model", "policy", "report", "syntax"];
const CONTEXTS: [&str; 4] = ["code", "graph", "history", "tools"];

fn source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn files_of(module: &str) -> Vec<PathBuf> {
    let root = source_root();
    if root.join(module).is_dir() {
        rust_files(&root.join(module))
    } else {
        vec![root.join(format!("{module}.rs"))]
    }
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

fn paths_after(path: &Path, prefix: &str) -> Vec<String> {
    let source = std::fs::read_to_string(path).expect("source readable");
    source
        .match_indices(prefix)
        .flat_map(|(start, _)| first_segments(&source[start + prefix.len()..]))
        .collect()
}

fn first_segments(rest: &str) -> Vec<String> {
    match rest.strip_prefix('{') {
        Some(group) => group
            .split_once('}')
            .map(|(inside, _)| inside)
            .unwrap_or_default()
            .split(',')
            .map(|entry| leading_identifier(entry.trim()))
            .collect(),
        None => vec![leading_identifier(rest)],
    }
}

fn leading_identifier(text: &str) -> String {
    text.chars()
        .take_while(|character| character.is_alphanumeric() || *character == '_')
        .collect()
}

fn assert_reaches_only(file: &Path, prefix: &str, allowed: &[&str], own: &str) {
    let label = if own.is_empty() {
        "the kernel".to_owned()
    } else {
        format!("the {own} context")
    };
    for module in paths_after(file, prefix) {
        assert!(
            allowed.contains(&module.as_str()) || module == own,
            "{} reaches {prefix}{module}, which is off limits to {label}",
            file.display()
        );
    }
}

#[test]
fn a_context_reaches_only_the_kernel_of_either_crate_and_its_own_core_context() {
    for context in CONTEXTS {
        for file in files_of(context) {
            assert_reaches_only(&file, "crate::", &KERNEL, context);
            assert_reaches_only(&file, "jabuti_core::", &CORE_KERNEL, context);
        }
    }
}

#[test]
fn the_kernel_reaches_no_context_except_where_it_composes_them() {
    for module in KERNEL.into_iter().filter(|module| *module != "main") {
        for file in files_of(module) {
            assert_reaches_only(&file, "crate::", &KERNEL, "");
            assert_reaches_only(&file, "jabuti_core::", &CORE_KERNEL, "");
        }
    }
}

#[test]
fn every_module_the_boundary_names_exists() {
    for module in KERNEL.into_iter().chain(CONTEXTS) {
        assert!(!files_of(module).is_empty(), "{module}");
        for file in files_of(module) {
            assert!(file.is_file(), "{}", file.display());
        }
    }
}
