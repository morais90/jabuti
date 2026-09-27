use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ignore::overrides::{Override, OverrideBuilder};
use jabuti_core::policy::Layers;

use super::workspace;
use crate::config::Layer;

pub(crate) fn assign(
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
        .map(|path| PathBuf::from(workspace::display(path, project)))
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
