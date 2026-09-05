use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use super::index::Edges;
use crate::model::Span;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Layers {
    pub of: BTreeMap<PathBuf, String>,
    pub allowed: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub from: PathBuf,
    pub to: PathBuf,
    pub at: Span,
    pub from_layer: String,
    pub to_layer: String,
}

pub fn violations(edges: &Edges, layers: &Layers) -> Vec<Violation> {
    let mut found = Vec::new();

    for ((from, to), at) in edges {
        let (Some(from_layer), Some(to_layer)) = (layers.of.get(from), layers.of.get(to)) else {
            continue;
        };
        if from_layer == to_layer {
            continue;
        }

        let permitted = layers
            .allowed
            .get(from_layer)
            .is_some_and(|allowed| allowed.contains(to_layer));
        if !permitted {
            found.push(Violation {
                from: from.clone(),
                to: to.clone(),
                at: *at,
                from_layer: from_layer.clone(),
                to_layer: to_layer.clone(),
            });
        }
    }

    found
}
