use std::path::{Path, PathBuf};

use anyhow::Result;
use jabuti_core::graph::index::{Index, Source};
use jabuti_core::lang::{self, LangSpec};
use jabuti_core::model::{Detail, Finding, Rule, RuleId, Severity, Span, Unreadable};

use super::sources;
use crate::config::Settings;
use crate::git::since::Changes;
use crate::project;

pub(crate) fn findings(
    paths: &[PathBuf],
    project: &Path,
    settings: &Settings,
    changes: &Changes,
) -> Result<(Vec<Finding>, Vec<Unreadable>)> {
    let rule = RuleId::Native(Rule::NewDependency);
    let base = crate::git::run(&["merge-base", "HEAD", changes.reference()])?
        .trim()
        .to_owned();
    let (indexed, unreadable) = sources::known(paths, project);
    let index = Index::of(&indexed);

    let mut found = Vec::new();
    for path in paths.iter().filter(|path| changes.covers(path)) {
        let Some(spec) = lang::detect(path) else {
            continue;
        };
        let Some(severity) = gating(settings, spec) else {
            continue;
        };
        let shown = project::display(path, project);

        let Some(inside) = changes.relative(path) else {
            continue;
        };
        let Some(before) = previous(&base, &inside) else {
            continue;
        };
        let Some(now) = sources::source_of(&shown, spec, sources::contents(path).as_deref()) else {
            continue;
        };
        let Some(then) = sources::source_of(&shown, spec, Some(&before)) else {
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

    Ok((found, unreadable))
}

fn gating(settings: &Settings, spec: &'static LangSpec) -> Option<Severity> {
    settings
        .policy
        .config_for(spec.id, Rule::NewDependency)
        .map(|config| config.severity)
        .filter(|severity| *severity != Severity::Off)
}

fn previous(base: &str, inside: &Path) -> Option<String> {
    crate::git::run(&["show", &format!("{base}:{}", inside.display())]).ok()
}

fn added(index: &Index, now: &Source, then: &Source) -> Vec<(PathBuf, Span)> {
    let before = index.targets(then);

    index
        .targets(now)
        .into_iter()
        .filter(|(target, _)| target != &now.path && !before.contains_key(target))
        .collect()
}
