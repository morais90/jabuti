mod drift;
mod layers;
mod sources;
mod surface;

use std::path::{Path, PathBuf};

use anyhow::Result;
use jabuti_core::lang::LanguageId;
use jabuti_core::model::{Finding, Rule, Severity, Unreadable};

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

fn reporting(settings: &Settings, language: LanguageId, rule: Rule) -> Option<Severity> {
    settings
        .policy
        .config_for(language, rule)
        .map(|config| config.severity)
        .filter(|severity| *severity != Severity::Off)
}
