use std::path::PathBuf;

use jabuti_core::graph::index::{Index, Source};
use jabuti_core::model::{Detail, Finding, Rule, RuleId, Span};

use super::Scan;

pub(crate) fn findings(scan: &Scan<'_>, index: &Index) -> Vec<Finding> {
    let rule = RuleId::Native(Rule::NewDependency);

    let mut found = Vec::new();
    for now in scan.sources {
        let Some(severity) = super::reporting(scan.settings, now.language, Rule::NewDependency)
        else {
            continue;
        };
        let Some(Some(then)) = scan.base.get(&now.path) else {
            continue;
        };

        for (target, at) in added(index, now, then) {
            found.push(Finding {
                rule: rule.clone(),
                severity,
                path: now.path.display().to_string(),
                span: at,
                subject: None,
                detail: Detail::Message {
                    message: format!("now depends on {}", target.display()),
                },
            });
        }
    }

    found
}

fn added(index: &Index, now: &Source, then: &Source) -> Vec<(PathBuf, Span)> {
    let before = index.targets(then);

    index
        .targets(now)
        .into_iter()
        .filter(|(target, _)| target != &now.path && !before.contains_key(target))
        .collect()
}
