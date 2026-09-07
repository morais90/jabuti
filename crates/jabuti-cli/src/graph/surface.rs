use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use jabuti_core::graph::facts::{self, FileFacts};
use jabuti_core::graph::index::{Index, Source};
use jabuti_core::graph::surface;
use jabuti_core::model::{Detail, Finding, Rule, RuleId, Unreadable};
use jabuti_core::syntax;

use super::sources;
use crate::config::Settings;
use crate::git::since::Changes;
use crate::project;

pub(crate) fn findings(
    paths: &[PathBuf],
    project: &Path,
    settings: &Settings,
    changes: &Changes,
    base: &BTreeMap<String, String>,
) -> Result<(Vec<Finding>, Vec<Unreadable>)> {
    if !settings.enabled(Rule::SpeculativeApi) {
        return Ok((Vec::new(), Vec::new()));
    }
    let universe = project::sources(&[project.to_path_buf()], &settings.exclude, project)?;
    let (all, unreadable) = sources::all(&universe, project);
    let roots = surface::roots(&all, &Index::of(&all));
    let opaque = opaque_texts(&unreadable, project);
    let scanned: Vec<String> = paths_shown(paths, project);

    let mut found = Vec::new();
    for source in all
        .iter()
        .filter(|source| considered(source, &scanned, changes))
    {
        let Some(severity) = super::reporting(settings, source.language, Rule::SpeculativeApi)
        else {
            continue;
        };
        let then = match at_base(base, source) {
            Base::Missing => None,
            Base::Unreadable => continue,
            Base::Facts(facts) => Some(facts),
        };

        for item in surface::speculative(source, then.as_ref(), &all, &roots)
            .into_iter()
            .filter(|item| {
                !opaque
                    .iter()
                    .any(|text| surface::mentioned_in(text, &item.name))
            })
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

    Ok((found, unreadable))
}

fn paths_shown(paths: &[PathBuf], project: &Path) -> Vec<String> {
    paths
        .iter()
        .map(|path| project::display(path, project))
        .collect()
}

fn considered(source: &Source, scanned: &[String], changes: &Changes) -> bool {
    let shown = source.path.display().to_string();

    scanned.contains(&shown)
        && changes.covers(&source.path)
        && !source.language.spec().is_test_path(&source.path)
}

enum Base {
    Missing,
    Unreadable,
    Facts(FileFacts),
}

fn at_base(base: &BTreeMap<String, String>, source: &Source) -> Base {
    let Some(before) = base.get(&source.path.display().to_string()) else {
        return Base::Missing;
    };

    match syntax::parse(before, source.language.spec()) {
        Ok(parsed) => Base::Facts(facts::facts(&parsed)),
        Err(_) => Base::Unreadable,
    }
}

fn opaque_texts(unreadable: &[Unreadable], project: &Path) -> Vec<String> {
    unreadable
        .iter()
        .filter_map(|file| sources::contents(&project.join(&file.path)))
        .collect()
}
