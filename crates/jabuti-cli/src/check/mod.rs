mod crossings;
mod examine;
mod plan;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use jabuti_core::authorship::{self, LeftOut, Origin};
use jabuti_core::catalog::Input;
use jabuti_core::code::Measured;
use jabuti_core::code::duplication::Before;
use jabuti_core::diff::{Moves, Placement, Reference};
use jabuti_core::graph::index::Source;
use jabuti_core::model::{Finding, Named, Span};
use jabuti_core::policy::Layers;
use jabuti_core::report::Outcome;
use jabuti_core::{code, diff, graph};

use self::examine::Base;
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
        roots,
        settings: &settings,
        paths: &paths,
        changes: changes.as_ref(),
        churn: &churn,
        touched: touched.as_ref(),
    };

    let mut outcome = judged(&run, notices)?;
    outcome.left_out.extend(excluded_by_project(&run, roots)?);
    if let Some(changes) = run.changes {
        changes
            .diff()
            .trim(&mut outcome.findings, &changes.placement(run.paths));
    }
    outcome.leave_out_unauthored();
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
    roots: &'a [PathBuf],
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
            before: None,
        }
    }

    fn extent(&self, request: &code::Scan<'_>) -> Result<Vec<PathBuf>> {
        if !self.plan.reads(Input::Graph) {
            return Ok(request.scope(self.paths).into_iter().collect());
        }

        workspace::extended(self.paths, &self.settings.exclude, self.root)
    }
}

fn excluded_by_project(run: &Run<'_>, roots: &[PathBuf]) -> Result<Vec<LeftOut>> {
    if run.settings.exclude.is_empty() {
        return Ok(Vec::new());
    }
    let excluded = workspace::excluded(roots, &run.settings.exclude, run.root)?;
    let named: BTreeSet<&PathBuf> = run.paths.iter().collect();
    let configured = run.root.join(crate::config::FILE_NAME);
    let shown = |path: &PathBuf| {
        let reconfigured = |changes: &Changes| changes.covers(&configured);
        run.changes
            .is_none_or(|changes| reconfigured(changes) || changes.covers(path))
    };

    Ok(excluded
        .iter()
        .filter(|path| !named.contains(path) && shown(path))
        .map(|path| LeftOut {
            path: workspace::display(path, run.root),
            origin: Origin::Excluded,
        })
        .collect())
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
    deleted: BTreeSet<String>,
    base: Base,
    placement: Placement,
}

impl Compared<'_> {
    fn touches(&self, path: &str, span: Span) -> bool {
        self.changes
            .diff()
            .touches(&self.placement.in_repository(Path::new(path)), span)
    }
}

struct Drawn<'a> {
    sources: &'a [Source],
    opaque: &'a [String],
    layers: Option<&'a Layers>,
    moves: &'a Moves,
}

fn judged(run: &Run<'_>, notices: &mut Vec<String>) -> Result<Outcome> {
    let compared = match run.changes {
        Some(changes) => Some(compare(run, changes)?),
        None => None,
    };
    let touches = |path: &str, span: Span| {
        compared
            .as_ref()
            .is_some_and(|compared| compared.touches(path, span))
    };
    let before = compared.as_ref().map(|compared| Before {
        blocks: compared.base.blocks.clone(),
        touches: &touches,
    });
    let request = code::Scan {
        before: before.as_ref(),
        ..run.request()
    };
    let extent = run.extent(&request)?;
    let examined = examine::files(run, &extent, &request);

    let (mut outcome, measured) = code::scan(examined.reviewed, &request);
    outcome.unreadable = examined.unreadable;
    outcome.left_out = examined.left_out;
    let moves = compared.as_ref().map_or_else(Moves::default, |compared| {
        moved(compared, &units(run, compared, &measured))
    });
    if let Some(compared) = &compared {
        let findings = std::mem::take(&mut outcome.findings);
        outcome.findings = worsened(compared, &moves, findings, notices);
    }
    outcome.findings.extend(crossings::hotspots(
        run.plan,
        &measured,
        &run.settings.policy,
    ));

    outcome
        .findings
        .extend(orchestrated(run, &measured, notices));

    let layers = layered(run, &extent, notices)?;
    let drawn = Drawn {
        sources: &examined.sources,
        opaque: &examined.opaque,
        layers: layers.as_ref(),
        moves: &moves,
    };
    let base_texts = compared.map_or_else(BTreeMap::new, |compared| compared.texts);
    outcome.findings.extend(graphed(run, &drawn, &base_texts));

    Ok(outcome)
}

fn orchestrated(run: &Run<'_>, measured: &[Measured], notices: &mut Vec<String>) -> Vec<Finding> {
    let tools = tools::Scan {
        project: run.root,
        paths: run.paths,
        touched: run.touched,
        settings: run.settings,
    };
    let (mut found, produced) = tools::findings(&tools, notices);
    if let Some(changes) = run.changes {
        let judged = crossings::Judged { changes, measured };
        found.extend(crossings::uncovered(&tools, judged, &produced, notices));
    }

    found
}

fn compare<'a>(run: &Run<'_>, changes: &'a Changes) -> Result<Compared<'a>> {
    let mut texts = changes.base_texts(run.paths, run.root)?;
    let mut deleted = BTreeSet::new();
    for (shown, source) in changes.deleted_texts(run.root)? {
        if gone_by_hand(run, &shown, &source)? {
            deleted.insert(shown.display().to_string());
            texts.insert(shown, source);
        }
    }
    let mut base = examine::base_findings(run, &texts, &run.request());
    for path in run.touched.into_iter().flatten() {
        base.blocks
            .entry(workspace::display(path, run.root))
            .or_default();
    }

    Ok(Compared {
        changes,
        texts,
        deleted,
        base,
        placement: changes.placement(run.paths),
    })
}

fn gone_by_hand(run: &Run<'_>, shown: &Path, source: &str) -> Result<bool> {
    let absolute = run.root.join(shown);
    let admitted = workspace::admits(&absolute, run.roots, &run.settings.exclude, run.root)?;

    Ok(admitted && authorship::origin(shown, source, &BTreeSet::new()).is_none())
}

fn moved(compared: &Compared<'_>, now: &BTreeMap<String, Vec<Named>>) -> Moves {
    let base: BTreeMap<String, Vec<Named>> = compared
        .base
        .units
        .iter()
        .filter(|(path, _)| now.contains_key(*path) || compared.deleted.contains(*path))
        .map(|(path, units)| (path.clone(), units.clone()))
        .collect();

    Moves::between(&base, now)
}

fn units(
    run: &Run<'_>,
    compared: &Compared<'_>,
    measured: &[Measured],
) -> BTreeMap<String, Vec<Named>> {
    let edited: BTreeSet<String> = run
        .touched
        .into_iter()
        .flatten()
        .map(|path| workspace::display(path, run.root))
        .chain(compared.base.units.keys().cloned())
        .collect();

    measured
        .iter()
        .filter(|file| edited.contains(&file.path))
        .filter(|file| !compared.base.unread.contains_key(&file.path))
        .map(|file| (file.path.clone(), file.units.clone()))
        .collect()
}

fn worsened(
    compared: &Compared<'_>,
    moves: &Moves,
    findings: Vec<Finding>,
    notices: &mut Vec<String>,
) -> Vec<Finding> {
    let base = &compared.base;
    let still_there = base
        .unread
        .iter()
        .filter(|(path, _)| !compared.deleted.contains(*path));
    for (path, reason) in still_there {
        notices.push(format!(
            "{path} could not be read as it was before the change ({reason}), so only its findings on lines the change touched are reported"
        ));
    }
    let placement = &compared.placement;
    let diff = compared.changes.diff();
    let place = |finding: &Finding| placement.in_repository(Path::new(&finding.path));
    let touches = |finding: &Finding| diff.touches(&place(finding), finding.span);

    let (unread, readable): (Vec<Finding>, Vec<Finding>) = findings
        .into_iter()
        .partition(|finding| base.unread.contains_key(&finding.path));
    let reference = Reference {
        findings: &base.findings,
        moves,
    };
    let mut kept = diff::worsened(readable, reference, touches, |finding| {
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
        moves: drawn.moves,
        layers: drawn.layers,
    })
}
