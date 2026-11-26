use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use crate::catalog::RuleId;
use crate::model::{Detail, Finding, Named, Span};

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
    deleted: BTreeSet<PathBuf>,
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
            deleted: reader.deleted,
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

    pub fn deleted(&self) -> impl Iterator<Item = &Path> {
        self.deleted.iter().map(PathBuf::as_path)
    }

    pub fn edits(&self, path: &Path) -> bool {
        self.covers(path) || self.hunks.contains_key(path)
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
type Kind = (RuleId, Option<String>, Option<String>, Carrier);
type Carrier = (Option<String>, String);

#[derive(Debug, Clone, Copy)]
pub struct Reference<'a> {
    pub findings: &'a [Finding],
    pub moves: &'a Moves,
}

struct Earlier<'a> {
    line: u32,
    measure: u32,
    finding: &'a Finding,
}

#[derive(Default)]
struct Settled<'a> {
    reported: Vec<Finding>,
    fresh: Vec<Finding>,
    unused: Vec<&'a Finding>,
}

impl Settled<'_> {
    fn absorb(&mut self, other: Self) {
        self.reported.extend(other.reported);
        self.fresh.extend(other.fresh);
        self.unused.extend(other.unused);
    }

    fn across(self, moves: &Moves, touched: &impl Fn(&Finding) -> bool) -> Vec<Finding> {
        let mut spare: BTreeMap<Kind, Vec<u32>> = BTreeMap::new();
        let departed = self.unused.into_iter().filter_map(|finding| {
            let carrier = moves.carried(finding)?;
            Some((kind(finding, carrier), measure(finding)))
        });
        for (kind, measure) in departed {
            spare.entry(kind).or_default().push(measure);
        }
        let (arrived, stayed): (Vec<Finding>, Vec<Finding>) = self
            .fresh
            .into_iter()
            .partition(|finding| moves.arrived(finding).is_some());
        let mut reported = self.reported;
        reported.extend(stayed);
        let mut arrivals: BTreeMap<Kind, Vec<Finding>> = BTreeMap::new();
        for finding in arrived {
            let unit = moves.arrived(&finding).unwrap_or_default();
            arrivals
                .entry(kind(&finding, unit))
                .or_default()
                .push(finding);
        }

        for (kind, group) in arrivals {
            let earlier = spare.remove(&kind).unwrap_or_default();
            reported.extend(match kind.2 {
                Some(_) => outnumbered(group, earlier.len(), touched),
                None => outgrown(group, earlier),
            });
        }

        reported
    }
}

pub fn worsened(
    findings: Vec<Finding>,
    reference: Reference<'_>,
    touched: impl Fn(&Finding) -> bool,
    moved: impl Fn(&Finding) -> u32,
) -> Vec<Finding> {
    let mut before: BTreeMap<Identity, Vec<Earlier<'_>>> = BTreeMap::new();
    for finding in reference.findings {
        before.entry(identity(finding)).or_default().push(Earlier {
            line: moved(finding),
            measure: measure(finding),
            finding,
        });
    }
    let mut groups: BTreeMap<Identity, Vec<Finding>> = BTreeMap::new();
    for finding in findings {
        groups.entry(identity(&finding)).or_default().push(finding);
    }

    let mut settled = Settled::default();
    for (identity, group) in groups {
        let earlier = before.remove(&identity).unwrap_or_default();
        settled.absorb(match identity.3 {
            Some(_) => surplus(group, earlier, &touched, reference.moves),
            None => grown(group, earlier),
        });
    }
    settled.unused.extend(
        before
            .into_values()
            .flatten()
            .map(|earlier| earlier.finding),
    );

    settled.across(reference.moves, &touched)
}

fn identity(finding: &Finding) -> Identity {
    (
        finding.rule.clone(),
        finding.path.clone(),
        finding.subject.clone(),
        message(finding),
    )
}

fn kind(finding: &Finding, carrier: Carrier) -> Kind {
    (
        finding.rule.clone(),
        finding.subject.clone(),
        message(finding),
        carrier,
    )
}

fn message(finding: &Finding) -> Option<String> {
    match &finding.detail {
        Detail::Message { message } => Some(message.clone()),
        Detail::Threshold { .. } => None,
    }
}

fn measure(finding: &Finding) -> u32 {
    match finding.detail {
        Detail::Threshold { measured, .. } => measured,
        Detail::Message { .. } => 0,
    }
}

fn grown(group: Vec<Finding>, earlier: Vec<Earlier<'_>>) -> Settled<'_> {
    let mut pairs: Vec<(u32, usize, usize)> = group
        .iter()
        .enumerate()
        .flat_map(|(current, finding)| {
            earlier
                .iter()
                .enumerate()
                .map(move |(was, then)| (finding.span.start_line.abs_diff(then.line), current, was))
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

    let mut settled = Settled::default();
    for (finding, was) in group.into_iter().zip(matched) {
        let Some(was) = was else {
            settled.fresh.push(finding);
            continue;
        };
        if measure(&finding) > earlier[was].measure {
            settled.reported.push(finding);
        }
    }
    settled.unused = earlier
        .into_iter()
        .zip(used)
        .filter(|(_, used)| !used)
        .map(|(then, _)| then.finding)
        .collect();

    settled
}

fn surplus<'a>(
    mut group: Vec<Finding>,
    mut earlier: Vec<Earlier<'a>>,
    touched: &impl Fn(&Finding) -> bool,
    moves: &Moves,
) -> Settled<'a> {
    let added = group.len().saturating_sub(earlier.len());
    let dropped = earlier.len().saturating_sub(group.len());
    group.sort_by_key(|finding| !touched(finding));
    group.truncate(added);
    earlier.sort_by_key(|then| moves.carried(then.finding).is_none());
    earlier.truncate(dropped);

    Settled {
        reported: Vec::new(),
        fresh: group,
        unused: earlier.into_iter().map(|then| then.finding).collect(),
    }
}

fn outnumbered(
    mut group: Vec<Finding>,
    earlier: usize,
    touched: &impl Fn(&Finding) -> bool,
) -> Vec<Finding> {
    let added = group.len().saturating_sub(earlier);
    group.sort_by_key(|finding| !touched(finding));
    group.truncate(added);

    group
}

fn outgrown(mut group: Vec<Finding>, mut earlier: Vec<u32>) -> Vec<Finding> {
    group.sort_by_key(|finding| Reverse(measure(finding)));
    earlier.sort_unstable_by_key(|measure| Reverse(*measure));

    group
        .into_iter()
        .enumerate()
        .filter(|(index, finding)| {
            earlier
                .get(*index)
                .is_none_or(|was| measure(finding) > *was)
        })
        .map(|(_, finding)| finding)
        .collect()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Moves {
    before: BTreeMap<String, Vec<Named>>,
    after: BTreeMap<String, Vec<Named>>,
}

type Key<'a> = (&'a Option<String>, &'a String);

impl Moves {
    pub fn between(
        base: &BTreeMap<String, Vec<Named>>,
        now: &BTreeMap<String, Vec<Named>>,
    ) -> Self {
        let departed = only_in(base, now);
        let arrived = only_in(now, base);
        let left: BTreeSet<Key<'_>> = departed.values().flatten().map(|unit| key(unit)).collect();
        let came: BTreeSet<Key<'_>> = arrived.values().flatten().map(|unit| key(unit)).collect();
        let moved: BTreeSet<Key<'_>> = left.intersection(&came).copied().collect();

        Self {
            before: keeping(&departed, &moved),
            after: keeping(&arrived, &moved),
        }
    }

    pub fn brought(&self, path: &str, name: &str, span: Span) -> bool {
        self.after.get(path).is_some_and(|units| {
            units.iter().any(|unit| {
                unit.name == name
                    && unit.span.start_line <= span.end_line
                    && span.start_line <= unit.span.end_line
            })
        })
    }

    fn carried(&self, finding: &Finding) -> Option<Carrier> {
        carrier(&self.before, &finding.path, finding.span)
    }

    fn arrived(&self, finding: &Finding) -> Option<Carrier> {
        carrier(&self.after, &finding.path, finding.span)
    }
}

fn key(unit: &Named) -> Key<'_> {
    (&unit.owner, &unit.name)
}

fn only_in<'a>(
    from: &'a BTreeMap<String, Vec<Named>>,
    to: &BTreeMap<String, Vec<Named>>,
) -> BTreeMap<&'a String, Vec<&'a Named>> {
    from.iter()
        .map(|(path, units)| {
            let there: BTreeSet<Key<'_>> = to.get(path).into_iter().flatten().map(key).collect();
            let gone = units
                .iter()
                .filter(|unit| !there.contains(&key(unit)))
                .collect();
            (path, gone)
        })
        .collect()
}

fn keeping(
    files: &BTreeMap<&String, Vec<&Named>>,
    moved: &BTreeSet<Key<'_>>,
) -> BTreeMap<String, Vec<Named>> {
    files
        .iter()
        .map(|(path, units)| {
            let kept = units
                .iter()
                .filter(|unit| moved.contains(&key(unit)))
                .map(|unit| (*unit).clone())
                .collect();
            ((*path).clone(), kept)
        })
        .collect()
}

fn carrier(files: &BTreeMap<String, Vec<Named>>, path: &str, span: Span) -> Option<Carrier> {
    files
        .get(path)?
        .iter()
        .find(|unit| unit.span.start_line <= span.start_line && span.end_line <= unit.span.end_line)
        .map(|unit| (unit.owner.clone(), unit.name.clone()))
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
    deleted: BTreeSet<PathBuf>,
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
            if self.current.is_none() {
                let gone = self.previous.strip_prefix("--- a/").map(trailing);
                self.deleted.extend(gone.map(PathBuf::from));
            }
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

    let spec = trailing(spec);

    Some(PathBuf::from(spec.strip_prefix("b/").unwrap_or(spec)))
}

fn trailing(spec: &str) -> &str {
    spec.strip_suffix('\t').unwrap_or(spec)
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
