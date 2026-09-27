use jabuti_core::catalog::{Rule, RuleId, Severity};
use jabuti_core::graph;
use jabuti_core::graph::index::{Edges, Index};
use jabuti_core::model::{Detail, Finding};

use super::Scan;
use crate::config::Settings;

pub(crate) fn findings(scan: &Scan<'_>, index: &Index) -> Vec<Finding> {
    let Some(severity) = reporting(scan.settings) else {
        return Vec::new();
    };
    let Some(layers) = scan.layers else {
        return Vec::new();
    };
    let edges = outgoing(scan, index);

    graph::layers::violations(&edges, layers)
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

fn reporting(settings: &Settings) -> Option<Severity> {
    if settings.layers.is_empty() {
        return None;
    }

    settings
        .policy
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
