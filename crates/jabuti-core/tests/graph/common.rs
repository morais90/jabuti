use std::path::{Path, PathBuf};

use jabuti_core::graph::facts;
use jabuti_core::graph::index::Source;
use jabuti_core::lang::LangSpec;
use jabuti_core::syntax;

pub(crate) fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph")
}

pub(crate) fn sources_under(relative: &str, spec: &'static LangSpec) -> Vec<Source> {
    let root = fixture_root().join(relative);
    let mut paths = Vec::new();
    gather(&root, &mut paths);
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            let source = std::fs::read_to_string(&path).expect("fixture readable");
            let parsed = syntax::parse(&source, spec)
                .unwrap_or_else(|_| panic!("fixture {} parses cleanly", path.display()));

            Source {
                path: path
                    .strip_prefix(&root)
                    .expect("under the root")
                    .to_path_buf(),
                language: spec.id,
                facts: facts::facts(&parsed),
            }
        })
        .collect()
}

fn gather(root: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(root)
        .expect("fixture directory")
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            gather(&path, found);
        } else {
            found.push(path);
        }
    }
}
