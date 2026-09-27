use std::path::{Path, PathBuf};

const KERNEL: [&str; 3] = ["check", "config", "main"];
const INPUTS: &str = "inputs";
const COMPOSERS: [&str; 2] = ["check", "main"];
const EXAMINE: &str = "check/examine";
const CROSSINGS: &str = "check/crossings";
const CORE_KERNEL: [&str; 7] = [
    "catalog", "diff", "lang", "model", "policy", "report", "syntax",
];
const TOUCHING_THE_WORLD: [&str; 9] = [
    "std::fs",
    "fs::",
    "std::process",
    "Command::new",
    "canonicalize",
    "current_dir",
    "read_to_string",
    ".modified()",
    "WalkBuilder",
];

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
fn the_kernel_reaches_no_context_except_where_it_composes_them() {
    let reachable: Vec<&str> = KERNEL.into_iter().chain([INPUTS]).collect();

    for module in KERNEL
        .into_iter()
        .filter(|module| !COMPOSERS.contains(module))
    {
        for file in files_of(module) {
            assert_reaches_only(&file, "crate::", &reachable, "");
            assert_reaches_only(&file, "jabuti_core::", &CORE_KERNEL, "");
        }
    }
}

#[test]
fn the_inputs_reach_the_configuration_and_the_core_and_nothing_that_analyses() {
    for file in files_of(INPUTS) {
        assert_reaches_only(&file, "crate::", &["config"], INPUTS);
    }
}

#[test]
fn only_the_inputs_the_configuration_the_examination_and_main_touch_the_world() {
    let allowed: Vec<PathBuf> = [INPUTS, "config", EXAMINE, "main"]
        .into_iter()
        .flat_map(files_of)
        .collect();

    for file in rust_files(&source_root()) {
        if allowed.contains(&file) {
            continue;
        }
        let source = std::fs::read_to_string(&file).expect("source readable");
        for call in TOUCHING_THE_WORLD {
            assert!(
                !source.contains(call),
                "{} calls {call} instead of receiving what inputs loaded",
                file.display()
            );
        }
    }
}

#[test]
fn only_the_examination_parses_a_source_file() {
    for file in rust_files(&source_root()) {
        if files_of(EXAMINE).contains(&file) {
            continue;
        }
        let source = std::fs::read_to_string(&file).expect("source readable");
        assert!(
            !source.contains("syntax::parse"),
            "{} parses on its own instead of receiving the corpus",
            file.display()
        );
    }
}

#[test]
fn only_the_examination_spreads_work_across_threads() {
    for file in rust_files(&source_root()) {
        if files_of(EXAMINE).contains(&file) {
            continue;
        }
        let source = std::fs::read_to_string(&file).expect("source readable");
        assert!(
            !source.contains("rayon"),
            "{} schedules work across threads outside the examination",
            file.display()
        );
    }
}

#[test]
fn the_pipeline_names_no_rule_and_leaves_that_to_the_crossings() {
    for file in files_of("check") {
        if files_of(CROSSINGS).contains(&file) {
            continue;
        }
        for named in paths_after(&file, "Rule::") {
            assert_eq!(
                named,
                "ALL",
                "{} names Rule::{named}, but the pipeline asks the catalog instead",
                file.display()
            );
        }
    }
}

#[test]
fn only_main_writes_to_the_terminal_and_every_other_module_hands_it_data() {
    for file in rust_files(&source_root()) {
        if file == source_root().join("main.rs") {
            continue;
        }
        let source = std::fs::read_to_string(&file).expect("source readable");
        for writer in ["eprintln!(", "eprint!(", "println!(", "print!("] {
            assert!(
                !source.contains(writer),
                "{} calls {writer} instead of returning what it has to say",
                file.display()
            );
        }
    }
}

#[test]
fn every_module_the_boundary_names_exists() {
    for module in KERNEL
        .into_iter()
        .chain([INPUTS, EXAMINE, CROSSINGS])
        .chain(COMPOSERS)
    {
        assert!(!files_of(module).is_empty(), "{module}");
        for file in files_of(module) {
            assert!(file.is_file(), "{}", file.display());
        }
    }
}
