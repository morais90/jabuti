use std::path::{Path, PathBuf};

use jabuti_core::graph::facts::{self, FileFacts};
use jabuti_core::graph::index::Source;
use jabuti_core::lang::LangSpec;
use jabuti_core::model::Unreadable;
use jabuti_core::{lang, syntax};

fn examine(path: &Path, project: &Path) -> Option<Result<Source, Unreadable>> {
    let spec = lang::detect(path)?;
    let shown = crate::project::display(path, project);

    let outcome = match std::fs::read_to_string(path).map(|source| facts_of(&source, spec)) {
        Ok(Ok(facts)) => Ok(Source {
            path: PathBuf::from(shown),
            language: spec.id,
            facts,
        }),
        Ok(Err(reason)) => Err(Unreadable {
            path: shown,
            reason: reason.to_string(),
        }),
        Err(_) => Err(Unreadable {
            path: shown,
            reason: "the file could not be read".to_owned(),
        }),
    };

    Some(outcome)
}

fn facts_of(source: &str, spec: &'static LangSpec) -> Result<FileFacts, syntax::SyntaxError> {
    syntax::parse(source, spec).map(|parsed| facts::facts(&parsed))
}

pub(crate) fn known(paths: &[PathBuf], project: &Path) -> (Vec<Source>, Vec<Unreadable>) {
    let mut sources = Vec::new();
    let mut unreadable = Vec::new();

    for path in paths {
        let Some(spec) = lang::detect(path) else {
            continue;
        };
        let shown = crate::project::display(path, project);

        match spec.id {
            lang::LanguageId::Rust => sources.push(Source {
                path: PathBuf::from(shown),
                language: spec.id,
                facts: FileFacts::default(),
            }),
            lang::LanguageId::Kotlin => match examine(path, project) {
                Some(Ok(source)) => sources.push(source),
                Some(Err(skipped)) => unreadable.push(skipped),
                None => {}
            },
        }
    }

    (sources, unreadable)
}

pub(crate) fn contents(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

pub(crate) fn source_of(
    shown: &str,
    spec: &'static LangSpec,
    contents: Option<&str>,
) -> Option<Source> {
    let facts = facts::facts(&syntax::parse(contents?, spec).ok()?);

    Some(Source {
        path: PathBuf::from(shown),
        language: spec.id,
        facts,
    })
}
