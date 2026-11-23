use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use crate::catalog::RuleId;
use crate::model::{Detail, Finding, Span};

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
    hunks: BTreeMap<PathBuf, Vec<Hunk>>,
    renames: BTreeMap<PathBuf, PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Hunk {
    old: (u32, u32),
    new: (u32, u32),
}

impl Diff {
    pub fn parse(unified: &str) -> Self {
        let mut reader = Reader::default();
        for line in unified.lines() {
            reader.read(line);
        }

        Self {
            touched: reader
                .touched
                .into_iter()
                .map(|(path, ranges)| (path, Touched::Lines(ranges)))
                .collect(),
            hunks: reader.hunks,
            renames: reader.renames,
        }
    }

    pub fn moved(&self, path: &Path, line: u32) -> u32 {
        let mut shift = 0_i64;
        for hunk in self.hunks.get(path).map_or(&[][..], Vec::as_slice) {
            let (old_start, old_count) = hunk.old;
            let first_after = old_start + old_count.max(1);
            if line >= first_after {
                shift += i64::from(hunk.new.1) - i64::from(old_count);
            } else if old_count > 0 && line >= old_start {
                return hunk.new.0.max(1);
            } else {
                break;
            }
        }

        u32::try_from(i64::from(line) + shift).unwrap_or(line)
    }

    pub fn origin<'a>(&'a self, path: &'a Path) -> &'a Path {
        self.renames.get(path).map_or(path, PathBuf::as_path)
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

type Identity = (RuleId, String, Option<String>, Option<String>);

pub fn worsened(
    findings: Vec<Finding>,
    base: &[Finding],
    touched: impl Fn(&Finding) -> bool,
    moved: impl Fn(&Finding) -> u32,
) -> Vec<Finding> {
    let mut before: BTreeMap<Identity, Vec<(u32, u32)>> = BTreeMap::new();
    for finding in base {
        before
            .entry(identity(finding))
            .or_default()
            .push((moved(finding), measure(finding)));
    }
    let mut groups: BTreeMap<Identity, Vec<Finding>> = BTreeMap::new();
    for finding in findings {
        groups.entry(identity(&finding)).or_default().push(finding);
    }

    groups
        .into_iter()
        .flat_map(|(identity, group)| {
            let earlier = before.remove(&identity).unwrap_or_default();
            match identity.3 {
                Some(_) => surplus(group, earlier.len(), &touched),
                None => grown(group, &earlier),
            }
        })
        .collect()
}

fn identity(finding: &Finding) -> Identity {
    let message = match &finding.detail {
        Detail::Message { message } => Some(message.clone()),
        Detail::Threshold { .. } => None,
    };

    (
        finding.rule.clone(),
        finding.path.clone(),
        finding.subject.clone(),
        message,
    )
}

fn measure(finding: &Finding) -> u32 {
    match finding.detail {
        Detail::Threshold { measured, .. } => measured,
        Detail::Message { .. } => 0,
    }
}

fn grown(group: Vec<Finding>, earlier: &[(u32, u32)]) -> Vec<Finding> {
    let mut pairs: Vec<(u32, usize, usize)> = group
        .iter()
        .enumerate()
        .flat_map(|(current, finding)| {
            earlier.iter().enumerate().map(move |(was, (line, _))| {
                (finding.span.start_line.abs_diff(*line), current, was)
            })
        })
        .collect();
    pairs.sort_unstable();

    let mut matched: Vec<Option<usize>> = vec![None; group.len()];
    let mut used = vec![false; earlier.len()];
    for (_, current, was) in pairs {
        if matched[current].is_none() && !used[was] {
            matched[current] = Some(was);
            used[was] = true;
        }
    }

    group
        .into_iter()
        .zip(matched)
        .filter(|(finding, was)| was.is_none_or(|was| measure(finding) > earlier[was].1))
        .map(|(finding, _)| finding)
        .collect()
}

fn surplus(
    mut group: Vec<Finding>,
    earlier: usize,
    touched: &impl Fn(&Finding) -> bool,
) -> Vec<Finding> {
    let added = group.len().saturating_sub(earlier);
    group.sort_by_key(|finding| !touched(finding));
    group.truncate(added);

    group
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

#[derive(Default)]
struct Reader<'a> {
    touched: BTreeMap<PathBuf, Vec<RangeInclusive<u32>>>,
    hunks: BTreeMap<PathBuf, Vec<Hunk>>,
    renames: BTreeMap<PathBuf, PathBuf>,
    renamed: Option<PathBuf>,
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
        } else if let Some(from) = line.strip_prefix("rename from ") {
            self.renamed = Some(PathBuf::from(from));
        } else if let (Some(to), Some(from)) =
            (line.strip_prefix("rename to "), self.renamed.take())
        {
            self.renames.insert(PathBuf::from(to), from);
        }

        self.previous = line;
    }

    fn open(&mut self, header: &str) {
        self.remaining = sides(header);
        let Some(path) = self.current.as_ref() else {
            return;
        };

        if let (Some(old), Some(new)) = (span_of(header, '-'), span_of(header, '+')) {
            self.hunks
                .entry(path.clone())
                .or_default()
                .push(Hunk { old, new });
        }
        if let Some(range) = added(header) {
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
