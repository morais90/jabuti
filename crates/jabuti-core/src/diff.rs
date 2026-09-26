use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use crate::model::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diff {
    touched: BTreeMap<PathBuf, Touched>,
}

impl Diff {
    pub fn parse(unified: &str) -> Self {
        Self {
            touched: hunks(unified),
        }
    }

    pub fn add_whole(&mut self, path: PathBuf) {
        self.touched.insert(path, Touched::Whole);
    }

    pub fn covers(&self, path: &Path) -> bool {
        self.touched.contains_key(path)
    }

    pub fn touches(&self, path: &Path, span: Span) -> bool {
        self.touched
            .get(path)
            .is_some_and(|touched| touched.covers(span))
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
