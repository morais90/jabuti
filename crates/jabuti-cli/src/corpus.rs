use std::path::{Path, PathBuf};

use jabuti_core::lang::{self, LangSpec, LanguageId};
use jabuti_core::syntax::{self, Parsed, SyntaxError};
use rayon::prelude::*;

use crate::project;

#[derive(Debug)]
pub(crate) struct Text {
    pub(crate) path: PathBuf,
    pub(crate) shown: String,
    pub(crate) spec: &'static LangSpec,
    pub(crate) source: String,
}

#[derive(Debug)]
pub(crate) struct File<T> {
    pub(crate) shown: String,
    pub(crate) language: LanguageId,
    pub(crate) outcome: Result<T, Rejected>,
}

#[derive(Debug)]
pub(crate) struct Rejected {
    pub(crate) reason: String,
    pub(crate) text: Option<String>,
}

pub(crate) fn examine<T: Send>(
    paths: &[PathBuf],
    project: &Path,
    derive: impl Fn(&Text, &Parsed<'_>) -> T + Sync,
) -> Vec<File<T>> {
    paths
        .par_iter()
        .filter_map(|path| examine_one(path, project, &derive))
        .collect()
}

fn examine_one<T>(
    path: &Path,
    project: &Path,
    derive: &impl Fn(&Text, &Parsed<'_>) -> T,
) -> Option<File<T>> {
    let spec = lang::detect(path)?;
    let shown = project::display(path, project);

    let outcome = match std::fs::read_to_string(path) {
        Ok(source) => derived(
            Text {
                path: path.to_path_buf(),
                shown: shown.clone(),
                spec,
                source,
            },
            derive,
        ),
        Err(_) => Err(Rejected {
            reason: "the file could not be read".to_owned(),
            text: None,
        }),
    };

    Some(File {
        shown,
        language: spec.id,
        outcome,
    })
}

fn derived<T>(text: Text, derive: &impl Fn(&Text, &Parsed<'_>) -> T) -> Result<T, Rejected> {
    match syntax::parse(&text.source, text.spec) {
        Ok(parsed) => Ok(derive(&text, &parsed)),
        Err(reason) => Err(Rejected {
            reason: reason.to_string(),
            text: Some(text.source),
        }),
    }
}

pub(crate) fn parsed<T>(
    source: &str,
    spec: &'static LangSpec,
    derive: impl FnOnce(&Parsed<'_>) -> T,
) -> Result<T, SyntaxError> {
    syntax::parse(source, spec).map(|parsed| derive(&parsed))
}
