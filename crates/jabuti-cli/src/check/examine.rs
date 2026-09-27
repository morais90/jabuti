use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use jabuti_core::catalog::Input;
use jabuti_core::graph::facts::{self, FileFacts};
use jabuti_core::graph::index::Source;
use jabuti_core::lang::{self, LangSpec, LanguageId};
use jabuti_core::model::Unreadable;
use jabuti_core::syntax::{self, Parsed, SyntaxError, Text};
use jabuti_core::{code, graph};
use rayon::prelude::*;

use super::Run;
use crate::inputs::git::since::Changes;
use crate::inputs::workspace;

#[derive(Debug)]
struct File<T> {
    shown: String,
    language: LanguageId,
    outcome: Result<T, Rejected>,
}

#[derive(Debug)]
struct Rejected {
    reason: String,
    text: Option<String>,
}

fn examine<T: Send>(
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
    let shown = workspace::display(path, project);

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

fn parsed<T>(
    source: &str,
    spec: &'static LangSpec,
    derive: impl FnOnce(&Parsed<'_>) -> T,
) -> Result<T, SyntaxError> {
    syntax::parse(source, spec).map(|parsed| derive(&parsed))
}

pub(super) struct Examined {
    pub(super) reviewed: Vec<code::Reviewed>,
    pub(super) sources: Vec<Source>,
    pub(super) unreadable: Vec<Unreadable>,
    pub(super) opaque: Vec<String>,
}

struct Derived {
    review: Option<code::Reviewed>,
    facts: Option<FileFacts>,
}

pub(super) fn files(run: &Run<'_>, extent: &[PathBuf], request: &code::Scan<'_>) -> Examined {
    let reviewed = request.scope(run.paths);
    let graphed = run.plan.reads(Input::Graph);
    let corpus = examine(extent, run.root, |text, parsed| Derived {
        review: reviewed.contains(&text.path).then(|| {
            let aliases = graph::aliases(parsed, text.spec.id, &run.settings.concepts);
            code::reviewed(text, parsed, &aliases, request)
        }),
        facts: graphed.then(|| facts::facts(parsed)),
    });

    let mut examined = Examined {
        reviewed: Vec::new(),
        sources: Vec::new(),
        unreadable: Vec::new(),
        opaque: Vec::new(),
    };
    for file in corpus {
        match file.outcome {
            Ok(derived) => {
                examined.reviewed.extend(derived.review);
                examined.sources.extend(
                    derived
                        .facts
                        .map(|facts| source(&file.shown, file.language, facts)),
                );
            }
            Err(rejected) => {
                examined.unreadable.push(Unreadable {
                    path: file.shown.clone(),
                    reason: rejected.reason,
                });
                examined.opaque.extend(rejected.text);
                if graphed {
                    examined
                        .sources
                        .push(source(&file.shown, file.language, FileFacts::default()));
                }
            }
        }
    }

    examined
}

pub(super) fn base_sources(
    changes: &Changes,
    paths: &[PathBuf],
    project: &Path,
) -> Result<BTreeMap<PathBuf, Option<Source>>> {
    Ok(changes
        .base_texts(paths, project)?
        .into_par_iter()
        .filter_map(|(shown, text)| base_source(shown, &text))
        .collect())
}

fn base_source(shown: PathBuf, text: &str) -> Option<(PathBuf, Option<Source>)> {
    let spec = lang::detect(&shown)?;
    let source = parsed(text, spec, facts::facts).ok().map(|facts| Source {
        path: shown.clone(),
        language: spec.id,
        facts,
    });

    Some((shown, source))
}

fn source(shown: &str, language: LanguageId, facts: FileFacts) -> Source {
    Source {
        path: PathBuf::from(shown),
        language,
        facts,
    }
}
