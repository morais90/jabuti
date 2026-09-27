mod drift;
mod layers;
mod surface;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use jabuti_core::catalog::{Rule, Severity};
use jabuti_core::graph::facts;
use jabuti_core::graph::index::{Index, Source};
use jabuti_core::lang::LanguageId;
use jabuti_core::model::Finding;
use jabuti_core::policy::{ConceptBindings, Layers};
use jabuti_core::syntax::Parsed;

use crate::config::Settings;

#[derive(Debug)]
pub(crate) struct Scan<'a> {
    pub(crate) requested: &'a BTreeSet<PathBuf>,
    pub(crate) sources: &'a [Source],
    pub(crate) opaque: &'a [String],
    pub(crate) base: &'a BTreeMap<PathBuf, Option<Source>>,
    pub(crate) settings: &'a Settings,
    pub(crate) compared: bool,
    pub(crate) layers: Option<&'a Layers>,
}

impl Scan<'_> {
    fn examines(&self, source: &Source) -> bool {
        self.requested.contains(&source.path)
    }
}

pub(crate) fn findings(scan: &Scan<'_>) -> Vec<Finding> {
    let index = Index::of(scan.sources);
    let mut found = Vec::new();

    if scan.compared {
        found.extend(drift::findings(scan, &index));
        found.extend(surface::findings(scan, &index));
    }
    found.extend(layers::findings(scan, &index));

    found
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
        .active_for(language, rule)
        .map(|config| config.severity)
}
