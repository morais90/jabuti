use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ignore::overrides::{Override, OverrideBuilder};
use jabuti_core::graph;
use jabuti_core::graph::index::{Edges, Index};
use jabuti_core::graph::layers::Layers;
use jabuti_core::model::{Detail, Finding, Rule, RuleId, Severity};

use super::Scan;
use crate::config::{Layer, Settings};
use crate::project;

pub(crate) fn findings(
    scan: &Scan<'_>,
    index: &Index,
    notices: &mut Vec<String>,
) -> Result<Vec<Finding>> {
    let Some(severity) = reporting(scan.settings) else {
        return Ok(Vec::new());
    };

    let layers = assign(&scan.settings.layers, scan.project, scan.paths, notices)?;
    let edges = outgoing(scan, index);

    let found = graph::layers::violations(&edges, &layers)
        .into_iter()
        .filter(|violation| {
            scan.changes
                .is_none_or(|changes| changes.touches(&violation.from, violation.at))
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

    Ok(found)
}

fn reporting(settings: &Settings) -> Option<Severity> {
    if settings.layers.is_empty() {
        return None;
    }

    settings
        .policy
        .active(Rule::LayerViolation)
        .map(|config| config.severity)
}

fn assign(
    declared: &[Layer],
    project: &Path,
    paths: &[PathBuf],
    notices: &mut Vec<String>,
) -> Result<Layers> {
    let mut layers = Layers::default();

    for layer in declared {
        let members = members_of(layer, project, paths)?;
        if members.is_empty() {
            notices.push(format!(
                "layer {} matches no file, so nothing is checked against it",
                layer.name
            ));
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
        .map(|path| PathBuf::from(project::display(path, project)))
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

fn outgoing(scan: &Scan<'_>, index: &Index) -> Edges {
    let mut edges = Edges::new();

    for source in scan.sources.iter().filter(|source| scan.examines(source)) {
        for (target, at) in index
            .targets(source)
            .into_iter()
            .filter(|(target, _)| target != &source.path)
        {
            edges.entry((source.path.clone(), target)).or_insert(at);
        }
    }

    edges
}
