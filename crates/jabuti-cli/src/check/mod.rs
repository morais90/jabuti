mod crossings;
mod examine;
mod plan;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use jabuti_core::catalog::Input;
use jabuti_core::graph::index::Source;
use jabuti_core::model::Finding;
use jabuti_core::policy::Layers;
use jabuti_core::report::Outcome;
use jabuti_core::{code, diff, graph};

use self::plan::Plan;
use crate::config::Settings;
use crate::inputs::git::since::{self, Changes};
use crate::inputs::{history, layers, tools, workspace};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Scope<'a> {
    Since(&'a str),
    Implied,
    All,
}

pub(crate) struct Verdict {
    pub(crate) root: PathBuf,
    pub(crate) outcome: Outcome,
    pub(crate) changes: Option<Changes>,
    pub(crate) reference: Option<String>,
    pub(crate) gating: bool,
}

pub(crate) fn verdict(
    roots: &[PathBuf],
    scope: Scope<'_>,
    notices: &mut Vec<String>,
) -> Result<Verdict> {
    let (root, settings) = workspace::discover()?;
    tools::known(&settings)?;
    let reference = match scope {
        Scope::Since(reference) => Some(reference.to_owned()),
        Scope::Implied => since::implied()?,
        Scope::All => None,
    };
    let changes = reference
        .as_deref()
        .map(|reference| {
            Changes::since(reference, &root).with_context(|| match scope {
                Scope::Implied => format!(
                    "the change was worked out from {reference}; name another reference with --since, or list the whole repository with --all"
                ),
                Scope::Since(_) | Scope::All => format!("comparing against {reference}"),
            })
        })
        .transpose()?;
    let plan = Plan::of(&settings, changes.is_some());
    notices.extend(plan.notices());

    let history = history::load(&settings, notices);
    let paths = workspace::sources(roots, &settings.exclude, &root)?;
    let churn = history::commits(history.as_ref(), &paths);
    let touched = changes.as_ref().map(|changes| touched_by(changes, &paths));
    let run = Run {
        plan,
        root: &root,
        settings: &settings,
        paths: &paths,
        changes: changes.as_ref(),
        churn: &churn,
        touched: touched.as_ref(),
    };

    let mut outcome = judged(&run, notices)?;
    if let Some(changes) = run.changes {
        changes
            .diff()
            .trim(&mut outcome.findings, &changes.placement(run.paths));
    }
    outcome.order();

    Ok(Verdict {
        root,
        outcome,
        changes,
        reference,
        gating: !matches!(scope, Scope::All),
    })
}

struct Run<'a> {
    plan: Plan<'a>,
    root: &'a Path,
    settings: &'a Settings,
    paths: &'a [PathBuf],
    changes: Option<&'a Changes>,
    churn: &'a BTreeMap<PathBuf, u32>,
    touched: Option<&'a BTreeSet<PathBuf>>,
}

impl Run<'_> {
    fn extent(&self, request: &code::Scan<'_>) -> Result<Vec<PathBuf>> {
        if !self.plan.reads(Input::Graph) {
            return Ok(request.scope(self.paths).into_iter().collect());
        }

        workspace::extended(self.paths, &self.settings.exclude, self.root)
    }
}

fn touched_by(changes: &Changes, paths: &[PathBuf]) -> BTreeSet<PathBuf> {
    paths
        .iter()
        .filter(|path| changes.covers(path))
        .cloned()
        .collect()
}

fn judged(run: &Run<'_>, notices: &mut Vec<String>) -> Result<Outcome> {
    let request = code::Scan {
        policy: &run.settings.policy,
        bindings: &run.settings.concepts,
        touched: run.touched,
        churn: run.churn,
    };
    let extent = run.extent(&request)?;
    let examined = examine::files(run, &extent, &request);

    let (mut outcome, measured) = code::scan(examined.reviewed, &request);
    outcome.unreadable = examined.unreadable;
    let base_texts = match run.changes {
        Some(changes) => changes.base_texts(run.paths, run.root)?,
        None => BTreeMap::new(),
    };
    if let Some(changes) = run.changes {
        outcome.findings = worsened(
            run,
            changes,
            &request,
            &base_texts,
            std::mem::take(&mut outcome.findings),
        );
    }
    outcome.findings.extend(crossings::hotspots(
        run.plan,
        &measured,
        &run.settings.policy,
    ));

    let tools = tools::Scan {
        project: run.root,
        paths: run.paths,
        settings: run.settings,
    };
    let (reported, produced) = tools::findings(&tools, notices)?;
    outcome.findings.extend(reported);
    if let Some(changes) = run.changes {
        outcome
            .findings
            .extend(crossings::uncovered(&tools, changes, produced, notices));
    }

    let layers = layered(run, &extent, notices)?;
    outcome.findings.extend(graphed(
        run,
        &examined.sources,
        &examined.opaque,
        layers.as_ref(),
        &base_texts,
    ));

    Ok(outcome)
}

fn worsened(
    run: &Run<'_>,
    changes: &Changes,
    request: &code::Scan<'_>,
    base_texts: &BTreeMap<PathBuf, String>,
    findings: Vec<Finding>,
) -> Vec<Finding> {
    let unhistoried = code::Scan {
        churn: &BTreeMap::new(),
        ..*request
    };
    let base = examine::base_findings(run, base_texts, &unhistoried);
    let placement = changes.placement(run.paths);

    let place = |finding: &Finding| placement.in_repository(Path::new(&finding.path));

    diff::worsened(
        findings,
        &base,
        |finding| changes.diff().touches(&place(finding), finding.span),
        |finding| {
            changes
                .diff()
                .moved(&place(finding), finding.span.start_line)
        },
    )
}

fn layered(run: &Run<'_>, extent: &[PathBuf], notices: &mut Vec<String>) -> Result<Option<Layers>> {
    if !run.plan.reads(Input::Layers) {
        return Ok(None);
    }

    layers::assign(&run.settings.layers, run.root, extent, notices).map(Some)
}

fn graphed(
    run: &Run<'_>,
    sources: &[Source],
    opaque: &[String],
    layers: Option<&Layers>,
    base_texts: &BTreeMap<PathBuf, String>,
) -> Vec<Finding> {
    let base = if run.plan.reads(Input::BaseRevision) {
        examine::base_sources(base_texts)
    } else {
        BTreeMap::new()
    };
    let requested: BTreeSet<PathBuf> = run
        .paths
        .iter()
        .filter(|path| run.touched.is_none_or(|touched| touched.contains(*path)))
        .map(|path| PathBuf::from(workspace::display(path, run.root)))
        .collect();

    graph::findings(&graph::Scan {
        requested: &requested,
        sources,
        opaque,
        base: &base,
        policy: &run.settings.policy,
        compared: run.changes.is_some(),
        layers,
    })
}
