use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use crate::model::{Finding, Span};

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

    pub fn trim(&self, findings: &mut Vec<Finding>, placement: &Placement) {
        findings.retain(|finding| {
            self.touches(
                &placement.in_repository(Path::new(&finding.path)),
                finding.span,
            )
        });
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Placement {
    pub project: PathBuf,
    pub aliases: BTreeMap<PathBuf, PathBuf>,
}

impl Placement {
    pub fn in_repository(&self, shown: &Path) -> PathBuf {
        self.aliases
            .get(shown)
            .cloned()
            .unwrap_or_else(|| self.project.join(shown))
    }
}

fn hunks(diff: &str) -> BTreeMap<PathBuf, Touched> {
    let mut reader = Reader::default();
    for line in diff.lines() {
        reader.read(line);
    }

    reader
        .touched
        .into_iter()
        .map(|(path, ranges)| (path, Touched::Lines(ranges)))
        .collect()
}

#[derive(Default)]
struct Reader<'a> {
    touched: BTreeMap<PathBuf, Vec<RangeInclusive<u32>>>,
    current: Option<PathBuf>,
    previous: &'a str,
    remaining: (u32, u32),
}

impl<'a> Reader<'a> {
    fn read(&mut self, line: &'a str) {
        if self.remaining != (0, 0) {
            self.remaining = consumed(self.remaining, line);
            return;
        }

        if let Some(spec) = line.strip_prefix("+++ ")
            && self.previous.starts_with("--- ")
        {
            self.current = target(spec);
        } else if let Some(header) = line.strip_prefix("@@ ") {
            self.open(header);
        }

        self.previous = line;
    }

    fn open(&mut self, header: &str) {
        self.remaining = sides(header);

        if let (Some(path), Some(range)) = (self.current.as_ref(), added(header)) {
            self.touched.entry(path.clone()).or_default().push(range);
        }
    }
}

fn target(spec: &str) -> Option<PathBuf> {
    if spec == "/dev/null" {
        return None;
    }

    Some(PathBuf::from(spec.strip_prefix("b/").unwrap_or(spec)))
}

fn added(header: &str) -> Option<RangeInclusive<u32>> {
    let (start, count) = span_of(header, '+')?;

    (count > 0).then(|| start..=start + count - 1)
}

fn sides(header: &str) -> (u32, u32) {
    let count = |sign| span_of(header, sign).map_or(0, |(_, count)| count);

    (count('-'), count('+'))
}

fn consumed((old, new): (u32, u32), line: &str) -> (u32, u32) {
    match line.chars().next() {
        Some('-') => (old.saturating_sub(1), new),
        Some('+') => (old, new.saturating_sub(1)),
        Some(' ') => (old.saturating_sub(1), new.saturating_sub(1)),
        _ => (old, new),
    }
}

fn span_of(header: &str, sign: char) -> Option<(u32, u32)> {
    let side = header
        .split_whitespace()
        .find(|part| part.starts_with(sign))?
        .trim_start_matches(sign);

    let mut numbers = side.split(',');
    let start: u32 = numbers.next()?.parse().ok()?;
    let count: u32 = match numbers.next() {
        Some(value) => value.parse().ok()?,
        None => 1,
    };

    Some((start, count))
}
