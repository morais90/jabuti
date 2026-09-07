use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use jabuti_core::graph::index::{Index, Source};
use jabuti_core::lang;
use jabuti_core::model::{Detail, Finding, Rule, RuleId, Span, Unreadable};

use super::sources;
use crate::config::Settings;
use crate::project;

pub(crate) fn findings(
    paths: &[PathBuf],
    project: &Path,
    settings: &Settings,
    base: &BTreeMap<String, String>,
) -> (Vec<Finding>, Vec<Unreadable>) {
    let rule = RuleId::Native(Rule::NewDependency);
    let (indexed, unreadable) = sources::known(paths, project);
    let index = Index::of(&indexed);

    let mut found = Vec::new();
    for path in paths {
        let Some(spec) = lang::detect(path) else {
            continue;
        };
        let Some(severity) = super::reporting(settings, spec.id, Rule::NewDependency) else {
            continue;
        };
        let shown = project::display(path, project);
        let Some(before) = base.get(&shown) else {
            continue;
        };
        let Some(now) = sources::source_of(&shown, spec, sources::contents(path).as_deref()) else {
            continue;
        };
        let Some(then) = sources::source_of(&shown, spec, Some(before)) else {
            continue;
        };

        for (target, at) in added(&index, &now, &then) {
            found.push(Finding {
                rule: rule.clone(),
                severity,
                path: shown.clone(),
                span: at,
                subject: None,
                detail: Detail::Message {
                    message: format!("now depends on {}", target.display()),
                },
            });
        }
    }

    (found, unreadable)
}

fn added(index: &Index, now: &Source, then: &Source) -> Vec<(PathBuf, Span)> {
    let before = index.targets(then);

    index
        .targets(now)
        .into_iter()
        .filter(|(target, _)| target != &now.path && !before.contains_key(target))
        .collect()
}
