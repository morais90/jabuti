mod drift;
mod layers;
mod surface;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use jabuti_core::graph::facts;
use jabuti_core::graph::index::{Index, Source};
use jabuti_core::lang::LanguageId;
use jabuti_core::model::{ConceptBindings, Finding, Rule, Severity};
use jabuti_core::syntax::Parsed;

use crate::config::Settings;
use crate::git::since::Changes;

#[derive(Debug)]
pub(crate) struct Scan<'a> {
    pub(crate) paths: &'a [PathBuf],
    pub(crate) requested: &'a BTreeSet<PathBuf>,
    pub(crate) sources: &'a [Source],
    pub(crate) opaque: &'a [String],
    pub(crate) base: &'a BTreeMap<PathBuf, Option<Source>>,
    pub(crate) project: &'a Path,
    pub(crate) settings: &'a Settings,
    pub(crate) changes: Option<&'a Changes>,
}

impl Scan<'_> {
    fn examines(&self, source: &Source) -> bool {
        self.requested.contains(&source.path)
            && self
                .changes
                .is_none_or(|changes| changes.covers(&source.path))
    }
}

pub(crate) fn findings(scan: &Scan<'_>) -> Result<Vec<Finding>> {
    let index = Index::of(scan.sources);
    let mut found = Vec::new();

    if scan.changes.is_some() {
        found.extend(drift::findings(scan, &index));
        found.extend(surface::findings(scan, &index));
    }
    found.extend(layers::findings(scan, &index)?);

    Ok(found)
}

pub(crate) fn aliases(
    parsed: &Parsed<'_>,
    language: LanguageId,
    bindings: &ConceptBindings,
) -> BTreeMap<String, String> {
    if bindings.is_empty_for(language) {
        return BTreeMap::new();
    }

    facts::aliases(parsed)
}

fn reporting(settings: &Settings, language: LanguageId, rule: Rule) -> Option<Severity> {
    settings
        .policy
        .config_for(language, rule)
        .map(|config| config.severity)
        .filter(|severity| *severity != Severity::Off)
}
