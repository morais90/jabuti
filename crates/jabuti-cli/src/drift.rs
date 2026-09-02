use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use jabuti_core::graph::{Index, Source};
use jabuti_core::lang::{self, LangSpec};
use jabuti_core::model::{Detail, FileFacts, Finding, Rule, RuleId, Severity, Span};
use jabuti_core::syntax;

use crate::config::Settings;
use crate::since::Changes;

pub(crate) fn findings(
    roots: &[PathBuf],
    settings: &Settings,
    changes: &Changes,
    reference: &str,
) -> Result<Vec<Finding>> {
    let rule = RuleId::Native(Rule::NewDependency);
    let base = crate::git::run(&["merge-base", "HEAD", reference])?
        .trim()
        .to_owned();
    let paths = crate::scan::sources(roots, &settings.exclude)?;
    let index = Index::of(&known(&paths));

    let mut found = Vec::new();
    for path in paths.iter().filter(|path| changes.covers(path)) {
        let Some(spec) = lang::detect(path) else {
            continue;
        };
        let Some(severity) = gating(settings, spec) else {
            continue;
        };
        let shown = crate::scan::display(path);

        let Some(inside) = changes.relative(path) else {
            continue;
        };
        let Some(before) = previous(&base, &inside) else {
            continue;
        };
        let Some(now) = source_of(&shown, spec, read(path).as_deref()) else {
            continue;
        };
        let Some(then) = source_of(&shown, spec, Some(&before)) else {
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

    Ok(found)
}

fn known(paths: &[PathBuf]) -> Vec<Source> {
    paths
        .iter()
        .filter_map(|path| {
            let spec = lang::detect(path)?;
            let shown = crate::scan::display(path);

            match spec.id {
                lang::LanguageId::Rust => Some(Source {
                    path: PathBuf::from(shown),
                    language: spec.id,
                    facts: FileFacts::default(),
                }),
                lang::LanguageId::Kotlin => source_of(&shown, spec, read(path).as_deref()),
            }
        })
        .collect()
}

fn read(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn source_of(shown: &str, spec: &'static LangSpec, contents: Option<&str>) -> Option<Source> {
    let facts = syntax::parse(contents?, spec).ok()?.facts();

    Some(Source {
        path: PathBuf::from(shown),
        language: spec.id,
        facts,
    })
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
    let before: BTreeSet<PathBuf> = index
        .targets(then)
        .into_iter()
        .map(|(target, _)| target)
        .collect();

    index
        .targets(now)
        .into_iter()
        .filter(|(target, _)| target != &now.path && !before.contains(target))
        .collect()
}
