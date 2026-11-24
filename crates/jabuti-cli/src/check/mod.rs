mod crossings;
mod examine;
mod plan;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use jabuti_core::catalog::Input;
use jabuti_core::code::Measured;
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

    let announced = changes.is_some() || matches!(scope, Scope::All);
    let history = history::load(plan.reads(Input::History) && announced, notices);
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
    outcome.set_context_apart();
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
    fn request(&self) -> code::Scan<'_> {
        code::Scan {
            policy: &self.settings.policy,
            bindings: &self.settings.concepts,
            touched: self.touched,
            churn: self.churn,
        }
    }

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

struct Compared<'a> {
    changes: &'a Changes,
    texts: BTreeMap<PathBuf, String>,
}

struct Drawn<'a> {
    sources: &'a [Source],
    opaque: &'a [String],
    layers: Option<&'a Layers>,
}

fn judged(run: &Run<'_>, notices: &mut Vec<String>) -> Result<Outcome> {
    let request = run.request();
    let extent = run.extent(&request)?;
    let examined = examine::files(run, &extent, &request);

    let (mut outcome, measured) = code::scan(examined.reviewed, &request);
    outcome.unreadable = examined.unreadable;
    let compared = match run.changes {
        Some(changes) => Some(Compared {
            changes,
            texts: changes.base_texts(run.paths, run.root)?,
        }),
        None => None,
    };
    if let Some(compared) = &compared {
        let findings = std::mem::take(&mut outcome.findings);
        outcome.findings = worsened(run, compared, findings, notices);
    }
    outcome.findings.extend(crossings::hotspots(
        run.plan,
        &measured,
        &run.settings.policy,
    ));

    outcome
        .findings
        .extend(orchestrated(run, &measured, notices)?);

    let layers = layered(run, &extent, notices)?;
    let drawn = Drawn {
        sources: &examined.sources,
        opaque: &examined.opaque,
        layers: layers.as_ref(),
    };
    let base_texts = compared.map_or_else(BTreeMap::new, |compared| compared.texts);
    outcome.findings.extend(graphed(run, &drawn, &base_texts));

    Ok(outcome)
}

fn orchestrated(
    run: &Run<'_>,
    measured: &[Measured],
    notices: &mut Vec<String>,
) -> Result<Vec<Finding>> {
    let tools = tools::Scan {
        project: run.root,
        paths: run.paths,
        settings: run.settings,
    };
    let (mut found, produced) = tools::findings(&tools, notices)?;
    if let Some(changes) = run.changes {
        let judged = crossings::Judged { changes, measured };
        found.extend(crossings::uncovered(&tools, judged, produced, notices));
    }

    Ok(found)
}

fn worsened(
    run: &Run<'_>,
    compared: &Compared<'_>,
    findings: Vec<Finding>,
    notices: &mut Vec<String>,
) -> Vec<Finding> {
    let base = examine::base_findings(run, &compared.texts, &run.request());
    for (path, reason) in &base.unread {
        notices.push(format!(
            "{path} could not be read as it was before the change ({reason}), so only its findings on lines the change touched are reported"
        ));
    }
    let placement = compared.changes.placement(run.paths);
    let diff = compared.changes.diff();
    let place = |finding: &Finding| placement.in_repository(Path::new(&finding.path));
    let touches = |finding: &Finding| diff.touches(&place(finding), finding.span);

    let (unread, readable): (Vec<Finding>, Vec<Finding>) = findings
        .into_iter()
        .partition(|finding| base.unread.contains_key(&finding.path));
    let mut kept = diff::worsened(readable, &base.findings, touches, |finding| {
        diff.moved(&place(finding), finding.span.start_line)
    });
    kept.extend(unread.into_iter().filter(|finding| touches(finding)));

    kept
}

fn layered(run: &Run<'_>, extent: &[PathBuf], notices: &mut Vec<String>) -> Result<Option<Layers>> {
    if !run.plan.reads(Input::Layers) {
        return Ok(None);
    }

    layers::assign(&run.settings.layers, run.root, extent, notices).map(Some)
}

fn graphed(
    run: &Run<'_>,
    drawn: &Drawn<'_>,
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
        sources: drawn.sources,
        opaque: drawn.opaque,
        base: &base,
        policy: &run.settings.policy,
        compared: run.changes.is_some(),
        layers: drawn.layers,
    })
}
