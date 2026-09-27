use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use jabuti_core::catalog::{Input, Rule, Scoping};
use jabuti_core::graph::facts::{self, FileFacts};
use jabuti_core::graph::index::Source;
use jabuti_core::history::hotspot::{self, FileSummary};
use jabuti_core::lang::{self, LanguageId};
use jabuti_core::model::{Finding, Unreadable};
use jabuti_core::report::Outcome;

use crate::git::since::Changes;
use crate::{code, config, corpus, graph, history, project, tools};

pub(crate) fn verdict(
    roots: &[PathBuf],
    since: Option<&str>,
    notices: &mut Vec<String>,
) -> Result<Outcome> {
    let (root, settings) = config::discover()?;
    tools::known(&settings)?;
    let changes = since
        .map(|reference| Changes::since(reference, &root))
        .transpose()?;
    notices.extend(scope_notices(&settings, since.is_some()));
    let history = history::load(&settings, notices);

    let paths = project::sources(roots, &settings.exclude, &root)?;
    let churn = history::commits(history.as_ref(), &paths);
    let scope = Scope {
        root: &root,
        settings: &settings,
        paths: &paths,
        changes: changes.as_ref(),
        churn: &churn,
    };

    let mut outcome = judged(&scope, notices)?;
    outcome.order();

    Ok(outcome)
}

struct Scope<'a> {
    root: &'a Path,
    settings: &'a config::Settings,
    paths: &'a [PathBuf],
    changes: Option<&'a Changes>,
    churn: &'a BTreeMap<PathBuf, u32>,
}

impl Scope<'_> {
    fn reads(&self, input: Input) -> bool {
        Rule::ALL
            .into_iter()
            .any(|rule| self.runs(rule) && rule.spec().inputs.contains(&input))
    }

    fn runs(&self, rule: Rule) -> bool {
        let spec = rule.spec();

        self.settings.enabled(rule)
            && spec.scoping.allows(self.changes.is_some())
            && spec.inputs.iter().all(|input| self.available(*input))
    }

    fn available(&self, input: Input) -> bool {
        match input {
            Input::Layers => !self.settings.layers.is_empty(),
            Input::BaseRevision | Input::Graph | Input::History => true,
        }
    }

    fn extent(&self, request: &code::Scan<'_>) -> Result<Vec<PathBuf>> {
        if !self.reads(Input::Graph) {
            return Ok(request.scope(self.paths).into_iter().collect());
        }

        project::extended(self.paths, &self.settings.exclude, self.root)
    }
}

fn judged(scope: &Scope<'_>, notices: &mut Vec<String>) -> Result<Outcome> {
    let request = code::Scan {
        policy: &scope.settings.policy,
        bindings: &scope.settings.concepts,
        changes: scope.changes,
        churn: scope.churn,
    };
    let extent = scope.extent(&request)?;
    let examined = examine(scope, &extent, &request);

    let (mut outcome, measured) = code::scan(examined.reviewed, &request);
    outcome.unreadable = examined.unreadable;
    if scope.runs(Rule::Hotspot) {
        outcome.findings.extend(hotspot::hotspots(
            &summaries(&measured),
            &scope.settings.policy,
        ));
    }
    outcome.findings.extend(tools::findings(
        &tools::Scan {
            here: &std::env::current_dir()?,
            project: scope.root,
            paths: scope.paths,
            settings: scope.settings,
            changes: scope.changes,
        },
        notices,
    ));
    outcome.findings.extend(graphed(
        scope,
        &extent,
        &examined.sources,
        &examined.opaque,
        notices,
    )?);

    Ok(outcome)
}

struct Derived {
    review: Option<code::Reviewed>,
    facts: Option<FileFacts>,
}

struct Examined {
    reviewed: Vec<code::Reviewed>,
    sources: Vec<Source>,
    unreadable: Vec<Unreadable>,
    opaque: Vec<String>,
}

fn examine(scope: &Scope<'_>, extent: &[PathBuf], request: &code::Scan<'_>) -> Examined {
    let reviewed = request.scope(scope.paths);
    let graphed = scope.reads(Input::Graph);
    let corpus = corpus::examine(extent, scope.root, |text, parsed| Derived {
        review: reviewed.contains(&text.path).then(|| {
            let aliases = graph::aliases(parsed, text.spec.id, &scope.settings.concepts);
            code::review(text, parsed, &aliases, request)
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

fn source(shown: &str, language: LanguageId, facts: FileFacts) -> Source {
    Source {
        path: PathBuf::from(shown),
        language,
        facts,
    }
}

fn graphed(
    scope: &Scope<'_>,
    extent: &[PathBuf],
    sources: &[Source],
    opaque: &[String],
    notices: &mut Vec<String>,
) -> Result<Vec<Finding>> {
    let base = match (scope.changes, scope.reads(Input::BaseRevision)) {
        (Some(changes), true) => base_sources(changes, scope.paths, scope.root)?,
        _ => BTreeMap::new(),
    };
    let requested: BTreeSet<PathBuf> = scope
        .paths
        .iter()
        .map(|path| PathBuf::from(project::display(path, scope.root)))
        .collect();

    graph::findings(
        &graph::Scan {
            paths: extent,
            requested: &requested,
            sources,
            opaque,
            base: &base,
            project: scope.root,
            settings: scope.settings,
            changes: scope.changes,
        },
        notices,
    )
}

fn base_sources(
    changes: &Changes,
    paths: &[PathBuf],
    project: &Path,
) -> Result<BTreeMap<PathBuf, Option<Source>>> {
    let mut sources = BTreeMap::new();

    for (shown, text) in changes.base_texts(paths, project)? {
        let Some(spec) = lang::detect(&shown) else {
            continue;
        };
        let source = corpus::parsed(&text, spec, facts::facts)
            .ok()
            .map(|facts| Source {
                path: shown.clone(),
                language: spec.id,
                facts,
            });
        sources.insert(shown, source);
    }

    Ok(sources)
}

fn scope_notices(settings: &config::Settings, scoped: bool) -> Vec<String> {
    let mut notices = Vec::new();

    for rule in Rule::ALL {
        match rule.spec().scoping {
            Scoping::Repository if scoped && settings.enabled(rule) => notices.push(format!(
                "{} ranks a whole repository, so it is not evaluated with --since",
                rule.id()
            )),
            Scoping::Change if !scoped && settings.gates(rule) => notices.push(format!(
                "{} compares against an earlier revision, so it needs --since",
                rule.id()
            )),
            Scoping::Any | Scoping::Change | Scoping::Repository => {}
        }
    }

    notices
}

fn summaries(measured: &[code::Measured]) -> Vec<FileSummary> {
    measured
        .iter()
        .map(|file| FileSummary {
            path: file.path.clone(),
            span: file.span,
            churn: file.churn,
            complexity: file.complexity,
        })
        .collect()
}
