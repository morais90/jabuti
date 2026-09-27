mod drift;
pub mod facts;
pub mod index;
mod lang;
pub mod layers;
pub mod surface;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use self::index::{Index, Source};
use crate::catalog::{Rule, Severity};
use crate::lang::LanguageId;
use crate::model::Finding;
use crate::policy::{ConceptBindings, Layers, Policy};
use crate::syntax::Parsed;

#[derive(Debug)]
pub struct Scan<'a> {
    pub requested: &'a BTreeSet<PathBuf>,
    pub sources: &'a [Source],
    pub opaque: &'a [String],
    pub base: &'a BTreeMap<PathBuf, Option<Source>>,
    pub policy: &'a Policy,
    pub compared: bool,
    pub layers: Option<&'a Layers>,
}

impl Scan<'_> {
    fn examines(&self, source: &Source) -> bool {
        self.requested.contains(&source.path)
    }
}

pub fn findings(scan: &Scan<'_>) -> Vec<Finding> {
    let index = Index::of(scan.sources);
    let mut found = Vec::new();

    if scan.compared {
        found.extend(drift::findings(scan, &index));
        found.extend(surface::findings(scan, &index));
    }
    found.extend(layers::findings(scan, &index));

    found
}

pub fn aliases(
    parsed: &Parsed<'_>,
    language: LanguageId,
    bindings: &ConceptBindings,
) -> BTreeMap<String, String> {
    if bindings.is_empty_for(language) {
        return BTreeMap::new();
    }

    facts::aliases(parsed)
}

fn reporting(policy: &Policy, language: LanguageId, rule: Rule) -> Option<Severity> {
    policy
        .active_for(language, rule)
        .map(|config| config.severity)
}
