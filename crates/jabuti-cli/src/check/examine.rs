use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use jabuti_core::authorship::{self, LeftOut, Origin};
use jabuti_core::catalog::Input;
use jabuti_core::code::duplication::{self, Blocks};
use jabuti_core::graph::facts::{self, FileFacts};
use jabuti_core::graph::index::Source;
use jabuti_core::lang::{self, LangSpec, LanguageId};
use jabuti_core::model::{Finding, Named, Unreadable};
use jabuti_core::syntax::{self, Parsed, SyntaxError, Text};
use jabuti_core::{code, graph};
use rayon::prelude::*;

use super::Run;
use crate::inputs::workspace;

#[derive(Debug)]
struct File<T> {
    path: PathBuf,
    shown: String,
    language: LanguageId,
    origin: Option<Origin>,
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
    derive: impl Fn(&Text, &Parsed<'_>, Option<Origin>) -> T + Sync,
) -> Vec<File<T>> {
    let translations = translations(paths, project);

    paths
        .par_iter()
        .filter_map(|path| examine_one(path, project, &translations, &derive))
        .collect()
}

fn translations(paths: &[PathBuf], project: &Path) -> BTreeSet<PathBuf> {
    let directories: BTreeSet<PathBuf> = paths
        .iter()
        .map(|path| PathBuf::from(workspace::display(path, project)))
        .filter_map(|shown| shown.parent().map(Path::to_path_buf))
        .filter(|directory| authorship::holds_translations(directory))
        .collect();
    let siblings: Vec<PathBuf> = directories
        .iter()
        .flat_map(|directory| {
            std::fs::read_dir(project.join(directory))
                .into_iter()
                .flatten()
                .flatten()
                .map(move |entry| directory.join(entry.file_name()))
        })
        .collect();

    authorship::translations(siblings.iter().map(PathBuf::as_path))
}

fn examine_one<T>(
    path: &Path,
    project: &Path,
    translations: &BTreeSet<PathBuf>,
    derive: &impl Fn(&Text, &Parsed<'_>, Option<Origin>) -> T,
) -> Option<File<T>> {
    let spec = lang::detect(path)?;
    let shown = workspace::display(path, project);
    let placed = authorship::placed(Path::new(&shown), translations);

    let Ok(source) = std::fs::read_to_string(path) else {
        return Some(File {
            path: path.to_path_buf(),
            shown,
            language: spec.id,
            origin: placed,
            outcome: Err(Rejected {
                reason: "the file could not be read".to_owned(),
                text: None,
            }),
        });
    };
    let origin = authorship::origin(Path::new(&shown), &source, translations);
    let text = Text {
        path: path.to_path_buf(),
        shown: shown.clone(),
        spec,
        source,
    };

    Some(File {
        path: path.to_path_buf(),
        shown,
        language: spec.id,
        origin,
        outcome: derived(text, |text, parsed| derive(text, parsed, origin)),
    })
}

fn derived<T>(text: Text, derive: impl Fn(&Text, &Parsed<'_>) -> T) -> Result<T, Rejected> {
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
    pub(super) left_out: Vec<LeftOut>,
}

struct Derived {
    review: Option<code::Reviewed>,
    facts: Option<FileFacts>,
}

pub(super) fn files(run: &Run<'_>, extent: &[PathBuf], request: &code::Scan<'_>) -> Examined {
    let reviewed = request.scope(run.paths);
    let graphed = run.plan.reads(Input::Graph);
    let corpus = examine(extent, run.root, |text, parsed, origin| Derived {
        review: (origin.is_none() && reviewed.contains(&text.path)).then(|| {
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
        left_out: Vec::new(),
    };
    let asked: BTreeSet<&PathBuf> = run.paths.iter().collect();
    let in_scope = |path: &PathBuf| {
        asked.contains(path) && run.touched.is_none_or(|touched| touched.contains(path))
    };
    for file in corpus {
        let in_scope = in_scope(&file.path);
        examined.take(file, in_scope, graphed);
    }

    examined
}

impl Examined {
    fn take(&mut self, file: File<Derived>, in_scope: bool, graphed: bool) {
        if let Some(origin) = file.origin
            && in_scope
        {
            self.left_out.push(LeftOut {
                path: file.shown.clone(),
                origin,
            });
        }
        match file.outcome {
            Ok(derived) => {
                self.reviewed.extend(derived.review);
                self.sources.extend(
                    derived
                        .facts
                        .map(|facts| source(&file.shown, file.language, facts)),
                );
            }
            Err(rejected) => {
                if file.origin.is_none() {
                    self.unreadable.push(Unreadable {
                        path: file.shown.clone(),
                        reason: rejected.reason,
                    });
                }
                self.opaque.extend(rejected.text);
                if graphed {
                    self.sources
                        .push(source(&file.shown, file.language, FileFacts::default()));
                }
            }
        }
    }
}

#[derive(Default)]
pub(super) struct Base {
    pub(super) findings: Vec<Finding>,
    pub(super) blocks: Blocks,
    pub(super) units: BTreeMap<String, Vec<Named>>,
    pub(super) unread: BTreeMap<String, String>,
}

pub(super) fn base_findings(
    run: &Run<'_>,
    texts: &BTreeMap<PathBuf, String>,
    request: &code::Scan<'_>,
) -> Base {
    type Read = Result<code::Parts, (String, String)>;
    let read: Vec<Read> = texts
        .par_iter()
        .filter_map(|(shown, source)| {
            let spec = lang::detect(shown)?;
            let text = Text {
                path: run.root.join(shown),
                shown: shown.to_string_lossy().into_owned(),
                spec,
                source: source.clone(),
            };
            Some(match syntax::parse(&text.source, spec) {
                Ok(parsed) => {
                    let aliases = graph::aliases(&parsed, spec.id, &run.settings.concepts);
                    Ok(code::reviewed(&text, &parsed, &aliases, request).into_parts())
                }
                Err(error) => Err((text.shown, error.to_string())),
            })
        })
        .collect();

    let mut base = Base::default();
    let mut fragments = Vec::new();
    for file in read {
        match file {
            Ok(parts) => {
                base.findings.extend(parts.findings);
                fragments.extend(parts.fragments);
                base.units.insert(parts.measured.path, parts.measured.units);
            }
            Err((shown, reason)) => {
                base.unread.insert(shown, reason);
            }
        }
    }
    base.blocks = duplication::blocks(&fragments);
    base
}

pub(super) fn base_sources(texts: &BTreeMap<PathBuf, String>) -> BTreeMap<PathBuf, Option<Source>> {
    texts
        .par_iter()
        .filter_map(|(shown, text)| base_source(shown.clone(), text))
        .collect()
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
