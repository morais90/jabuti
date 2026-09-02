use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use jabuti_core::graph::{self, Source};
use jabuti_core::model::Unreadable;
use jabuti_core::{lang, report, syntax};
use rayon::prelude::*;

use crate::config::Settings;
use crate::since::Changes;

pub(crate) fn impact(
    roots: &[PathBuf],
    settings: &Settings,
    changes: &Changes,
    limit: usize,
) -> Result<String> {
    let paths = crate::scan::sources(roots, &settings.exclude)?;
    let (sources, unreadable) = read(&paths);
    let dependents = reversed(&graph::edges(&sources));

    let touched: Vec<PathBuf> = paths
        .iter()
        .filter(|path| changes.covers(path))
        .map(|path| PathBuf::from(crate::scan::display(path)))
        .collect();

    let mut rendered = render(&touched, &dependents, limit);
    rendered.push_str(&report::unreadable(&unreadable, limit));

    Ok(rendered)
}

fn read(paths: &[PathBuf]) -> (Vec<Source>, Vec<Unreadable>) {
    let reviewed: Vec<Result<Source, Unreadable>> = paths.par_iter().filter_map(examine).collect();
    let mut sources = Vec::new();
    let mut unreadable = Vec::new();

    for outcome in reviewed {
        match outcome {
            Ok(source) => sources.push(source),
            Err(skipped) => unreadable.push(skipped),
        }
    }

    (sources, unreadable)
}

fn examine(path: &PathBuf) -> Option<Result<Source, Unreadable>> {
    let spec = lang::detect(path)?;
    let shown = crate::scan::display(path);

    let outcome = match std::fs::read_to_string(path).map(|source| facts_of(&source, spec)) {
        Ok(Ok(facts)) => Ok(Source {
            path: PathBuf::from(shown),
            language: spec.id,
            facts,
        }),
        Ok(Err(reason)) => Err(Unreadable {
            path: shown,
            reason: reason.to_string(),
        }),
        Err(_) => Err(Unreadable {
            path: shown,
            reason: "the file could not be read".to_owned(),
        }),
    };

    Some(outcome)
}

fn facts_of(
    source: &str,
    spec: &'static lang::LangSpec,
) -> Result<jabuti_core::model::FileFacts, syntax::SyntaxError> {
    syntax::parse(source, spec).map(|parsed| parsed.facts())
}

fn reversed(edges: &graph::Edges) -> BTreeMap<PathBuf, BTreeSet<PathBuf>> {
    let mut reversed: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();

    for (from, to) in edges.keys() {
        reversed.entry(to.clone()).or_default().insert(from.clone());
    }

    reversed
}

fn reaching(start: &Path, dependents: &BTreeMap<PathBuf, BTreeSet<PathBuf>>) -> BTreeSet<PathBuf> {
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([start.to_path_buf()]);

    while let Some(current) = queue.pop_front() {
        for next in dependents.get(&current).into_iter().flatten() {
            if next != start && seen.insert(next.clone()) {
                queue.push_back(next.clone());
            }
        }
    }

    seen
}

fn render(
    touched: &[PathBuf],
    dependents: &BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    limit: usize,
) -> String {
    let mut rendered = String::new();
    let reached: BTreeMap<&PathBuf, BTreeSet<PathBuf>> = touched
        .iter()
        .map(|path| (path, reaching(path, dependents)))
        .collect();

    let total: BTreeSet<&PathBuf> = reached.values().flatten().collect();

    writeln!(
        rendered,
        "{} changed, {} reached.",
        plural(touched.len(), "file"),
        plural(total.len(), "file")
    )
    .expect("writing to a string never fails");

    for (path, dependents) in reached.iter().filter(|(_, set)| !set.is_empty()) {
        writeln!(rendered, "\n{}", path.display()).expect("writing to a string never fails");
        for dependent in dependents.iter().take(limit) {
            writeln!(rendered, "  {}", dependent.display())
                .expect("writing to a string never fails");
        }

        let hidden = dependents.len().saturating_sub(limit);
        if hidden > 0 {
            writeln!(rendered, "  {} not shown.", plural(hidden, "further file"))
                .expect("writing to a string never fails");
        }
    }

    rendered
}

fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("{count} {noun}")
    } else {
        format!("{count} {noun}s")
    }
}
