use std::path::PathBuf;

use super::Scan;
use super::index::{Edges, Index};
use crate::catalog::{Rule, RuleId, Severity};
use crate::model::{Detail, Finding, Span};
use crate::policy::{Layers, Policy};

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

pub(crate) fn findings(scan: &Scan<'_>, index: &Index) -> Vec<Finding> {
    let Some(severity) = reporting(scan.policy) else {
        return Vec::new();
    };
    let Some(layers) = scan.layers else {
        return Vec::new();
    };
    let edges = outgoing(scan, index);

    violations(&edges, layers)
        .into_iter()
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
        .collect()
}

fn reporting(policy: &Policy) -> Option<Severity> {
    policy
        .active(Rule::LayerViolation)
        .map(|config| config.severity)
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
