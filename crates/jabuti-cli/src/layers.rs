use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ignore::overrides::{Override, OverrideBuilder};
use jabuti_core::graph::{self, Edges, Index, Layers};
use jabuti_core::lang;
use jabuti_core::model::{Detail, Finding, Rule, RuleId, Severity, Span, Unreadable};

use crate::config::{Layer, Settings};
use crate::since::Changes;

pub(crate) fn findings(
    roots: &[PathBuf],
    project: &Path,
    settings: &Settings,
    changes: Option<&Changes>,
) -> Result<(Vec<Finding>, Vec<Unreadable>)> {
    let Some(severity) = reporting(settings) else {
        return Ok((Vec::new(), Vec::new()));
    };

    let paths = crate::scan::sources(roots, &settings.exclude)?;
    let layers = assign(&settings.layers, project, &paths)?;
    let (indexed, unreadable) = crate::graph::known(&paths);
    let edges = outgoing(&paths, &Index::of(&indexed), changes);

    let found = graph::violations(&edges, &layers)
        .into_iter()
        .filter(|violation| {
            changes.is_none_or(|changes| changes.touches(&violation.from, violation.at))
        })
        .map(|violation| Finding {
            rule: RuleId::Native(Rule::LayerViolation),
            severity,
            path: violation.from.display().to_string(),
            span: violation.at,
            subject: None,
            detail: Detail::Message {
                message: format!(
                    "{} may not depend on {} ({})",
                    violation.from_layer,
                    violation.to_layer,
                    violation.to.display()
                ),
            },
        })
        .collect();

    Ok((found, unreadable))
}

fn reporting(settings: &Settings) -> Option<Severity> {
    if settings.layers.is_empty() {
        return None;
    }

    settings
        .policy
        .config(Rule::LayerViolation)
        .map(|config| config.severity)
        .filter(|severity| *severity != Severity::Off)
}

fn assign(declared: &[Layer], project: &Path, paths: &[PathBuf]) -> Result<Layers> {
    let mut layers = Layers::default();

    for layer in declared {
        let members = members_of(layer, project, paths)?;
        if members.is_empty() {
            eprintln!(
                "jabuti: layer {} matches no file, so nothing is checked against it",
                layer.name
            );
        }
        claim(&mut layers, layer, members)?;
    }

    Ok(layers)
}

fn claim(layers: &mut Layers, layer: &Layer, members: Vec<PathBuf>) -> Result<()> {
    for path in members {
        if let Some(other) = layers.of.insert(path.clone(), layer.name.clone()) {
            bail!(
                "{} is in both the {other} and the {} layer, and a file can belong to only one",
                path.display(),
                layer.name
            );
        }
    }
    layers.allowed.insert(
        layer.name.clone(),
        layer.depends_on.iter().cloned().collect(),
    );

    Ok(())
}

fn members_of(layer: &Layer, project: &Path, paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let selects = matcher_for(layer, project)?;

    Ok(paths
        .iter()
        .filter(|path| {
            path.canonicalize()
                .is_ok_and(|absolute| selects.matched(&absolute, false).is_whitelist())
        })
        .map(|path| PathBuf::from(crate::scan::display(path)))
        .collect())
}

fn matcher_for(layer: &Layer, project: &Path) -> Result<Override> {
    let mut builder = OverrideBuilder::new(project);
    for pattern in &layer.paths {
        builder
            .add(pattern)
            .with_context(|| format!("invalid path {pattern} in layer {}", layer.name))?;
    }

    builder.build().context("building layer matcher")
}

fn outgoing(paths: &[PathBuf], index: &Index, changes: Option<&Changes>) -> Edges {
    let mut edges = Edges::new();

    for path in paths {
        if changes.is_some_and(|changes| !changes.covers(path)) {
            continue;
        }
        for (from, target, at) in edges_from(path, index) {
            edges.entry((from, target)).or_insert(at);
        }
    }

    edges
}

fn edges_from(path: &Path, index: &Index) -> Vec<(PathBuf, PathBuf, Span)> {
    let Some(spec) = lang::detect(path) else {
        return Vec::new();
    };
    let shown = crate::scan::display(path);
    let Some(source) =
        crate::graph::source_of(&shown, spec, crate::graph::contents(path).as_deref())
    else {
        return Vec::new();
    };

    index
        .targets(&source)
        .into_iter()
        .filter(|(target, _)| target != &source.path)
        .map(|(target, at)| (source.path.clone(), target, at))
        .collect()
}
