use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use anyhow::Result;
use jabuti_core::model::Span;

#[derive(Debug)]
enum Touched {
    Whole,
    Lines(Vec<RangeInclusive<u32>>),
}

impl Touched {
    fn covers(&self, span: Span) -> bool {
        match self {
            Self::Whole => true,
            Self::Lines(ranges) => ranges
                .iter()
                .any(|range| *range.start() <= span.end_line && span.start_line <= *range.end()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Changes {
    root: PathBuf,
    touched: BTreeMap<PathBuf, Touched>,
}

impl Changes {
    pub(crate) fn since(reference: &str) -> Result<Self> {
        let root = PathBuf::from(crate::git::run(&["rev-parse", "--show-toplevel"])?.trim());
        let diff = crate::git::run(&["diff", "--unified=0", "--merge-base", reference])?;
        let untracked = crate::git::run(&["ls-files", "--others", "--exclude-standard"])?;

        let mut touched = hunks(&diff);
        for path in untracked.lines().filter(|line| !line.is_empty()) {
            touched.insert(PathBuf::from(path), Touched::Whole);
        }

        Ok(Self {
            root: root.canonicalize().unwrap_or(root),
            touched,
        })
    }

    pub(crate) fn covers(&self, path: &Path) -> bool {
        self.entry(path).is_some()
    }

    pub(crate) fn touches(&self, path: &Path, span: Span) -> bool {
        self.entry(path).is_some_and(|touched| touched.covers(span))
    }

    fn entry(&self, path: &Path) -> Option<&Touched> {
        let absolute = path.canonicalize().ok()?;
        let relative = absolute.strip_prefix(&self.root).ok()?;

        self.touched.get(relative)
    }
}

fn hunks(diff: &str) -> BTreeMap<PathBuf, Touched> {
    let mut touched: BTreeMap<PathBuf, Vec<RangeInclusive<u32>>> = BTreeMap::new();
    let mut current = None;
    let mut previous = "";

    for line in diff.lines() {
        if let Some(spec) = line.strip_prefix("+++ ")
            && previous.starts_with("--- ")
        {
            current = target(spec);
        } else if let Some(header) = line.strip_prefix("@@ ")
            && let (Some(path), Some(range)) = (current.as_ref(), added(header))
        {
            touched.entry(path.clone()).or_default().push(range);
        }

        previous = line;
    }

    touched
        .into_iter()
        .map(|(path, ranges)| (path, Touched::Lines(ranges)))
        .collect()
}

fn target(spec: &str) -> Option<PathBuf> {
    if spec == "/dev/null" {
        return None;
    }

    Some(PathBuf::from(spec.strip_prefix("b/").unwrap_or(spec)))
}

fn added(header: &str) -> Option<RangeInclusive<u32>> {
    let addition = header
        .split_whitespace()
        .find(|part| part.starts_with('+'))?
        .trim_start_matches('+');

    let mut numbers = addition.split(',');
    let start: u32 = numbers.next()?.parse().ok()?;
    let count: u32 = match numbers.next() {
        Some(value) => value.parse().ok()?,
        None => 1,
    };

    if count == 0 {
        return None;
    }

    Some(start..=start + count - 1)
}
