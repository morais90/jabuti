use jabuti_core::catalog::{Rule, RuleId};
use jabuti_core::graph::index::{Index, Source};
use jabuti_core::graph::surface;
use jabuti_core::model::{Detail, Finding};

use super::Scan;

pub(crate) fn findings(scan: &Scan<'_>, index: &Index) -> Vec<Finding> {
    if !scan.settings.enabled(Rule::SpeculativeApi) {
        return Vec::new();
    }
    let roots = surface::roots(scan.sources, index);

    let mut found = Vec::new();
    for source in scan
        .sources
        .iter()
        .filter(|source| considered(source, scan))
    {
        let Some(severity) = super::reporting(scan.settings, source.language, Rule::SpeculativeApi)
        else {
            continue;
        };
        let then = match scan.base.get(&source.path) {
            None => None,
            Some(None) => continue,
            Some(Some(then)) => Some(&then.facts),
        };

        for item in surface::speculative(source, then, scan.sources, &roots)
            .into_iter()
            .filter(|item| !mentioned_in_unreadable(scan, &item.name))
        {
            found.push(Finding {
                rule: RuleId::Native(Rule::SpeculativeApi),
                severity,
                path: item.path.display().to_string(),
                span: item.span,
                subject: Some(item.name),
                detail: Detail::Message {
                    message: "public, and nothing references it".to_owned(),
                },
            });
        }
    }

    found
}

fn considered(source: &Source, scan: &Scan<'_>) -> bool {
    scan.examines(source) && !source.language.spec().is_test_path(&source.path)
}

fn mentioned_in_unreadable(scan: &Scan<'_>, name: &str) -> bool {
    scan.opaque
        .iter()
        .any(|text| surface::mentioned_in(text, name))
}
