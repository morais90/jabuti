mod drift;
mod layers;
mod sources;
mod surface;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use jabuti_core::lang::{self, LanguageId};
use jabuti_core::model::{ConceptBindings, Finding, Rule, Severity, Unreadable};
use jabuti_core::{graph, syntax};

use crate::config::Settings;
use crate::git::since::Changes;

pub(crate) fn findings(
    paths: &[PathBuf],
    project: &Path,
    settings: &Settings,
    changes: Option<&Changes>,
) -> Result<(Vec<Finding>, Vec<Unreadable>)> {
    let (mut found, mut unreadable) = (Vec::new(), Vec::new());

    if let Some(changes) = changes {
        let (drifted, skipped) = drift::findings(paths, project, settings, changes)?;
        found.extend(drifted);
        unreadable.extend(skipped);
        let (unused, skipped) = surface::findings(paths, project, settings, changes)?;
        found.extend(unused);
        unreadable.extend(skipped);
    }

    let (crossed, skipped) = layers::findings(paths, project, settings, changes)?;
    found.extend(crossed);
    unreadable.extend(skipped);

    Ok((found, unreadable))
}

pub(crate) fn aliases(
    paths: &[PathBuf],
    bindings: &ConceptBindings,
) -> BTreeMap<PathBuf, BTreeMap<String, String>> {
    if bindings.is_empty() {
        return BTreeMap::new();
    }

    let mut found = BTreeMap::new();

    for path in paths {
        let Some(spec) = lang::detect(path) else {
            continue;
        };
        if bindings.is_empty_for(spec.id) {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(parsed) = syntax::parse(&source, spec) else {
            continue;
        };
        let aliases = graph::facts::aliases(&parsed);
        if !aliases.is_empty() {
            found.insert(path.clone(), aliases);
        }
    }

    found
}

fn reporting(settings: &Settings, language: LanguageId, rule: Rule) -> Option<Severity> {
    settings
        .policy
        .config_for(language, rule)
        .map(|config| config.severity)
        .filter(|severity| *severity != Severity::Off)
}
