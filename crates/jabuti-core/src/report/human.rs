use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt::Write;
use std::path::{Component, Path};

use super::sarif::encode_component;
use super::trail::{Line, Trail};
use super::{Judgement, Rhythm, Scanned, WEEKS, json, plural, write_detail};
use crate::model::{Detail, Finding, Unreadable};

const BAR_WIDTH: u32 = 12;
const SUBJECT_WIDTH: usize = 24;
const CODE_WIDTH: usize = 72;
const COMMITS_SHOWN: usize = 5;
const SPARK: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug, Clone, Copy)]
pub struct View<'a> {
    pub since: Option<&'a str>,
    pub color: bool,
    pub links: Option<&'a Path>,
    pub rhythm: Option<&'a BTreeMap<String, Rhythm>>,
    pub trail: Option<&'a Trail>,
}

pub fn interactive(terminal: bool, term: Option<&OsStr>) -> bool {
    terminal && term.is_none_or(|name| name != "dumb")
}

pub fn colored(interactive: bool, no_color: Option<&OsStr>) -> bool {
    let declined = no_color.is_some_and(|value| !value.is_empty());
    interactive && !declined
}

#[derive(Debug, Clone, Copy)]
enum Tone {
    Amber,
    Scale,
    Bone,
    Dim,
    Moss,
}

impl Tone {
    fn rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Amber => (0xe8, 0xb3, 0x3a),
            Self::Scale => (0xe2, 0x55, 0x2a),
            Self::Bone => (0xd9, 0xcf, 0xbd),
            Self::Dim => (0x7d, 0x73, 0x66),
            Self::Moss => (0x9b, 0xb0, 0x68),
        }
    }
}

struct Paint<'a> {
    view: View<'a>,
}

impl Paint<'_> {
    fn plain(&self, tone: Tone, text: &str) -> String {
        self.styled(tone, text, "")
    }

    fn strong(&self, tone: Tone, text: &str) -> String {
        self.styled(tone, text, "1;")
    }

    fn styled(&self, tone: Tone, text: &str, weight: &str) -> String {
        let text = printable(text);
        if !self.view.color || text.is_empty() {
            return text;
        }
        let (red, green, blue) = tone.rgb();
        format!("\x1b[{weight}38;2;{red};{green};{blue}m{text}\x1b[0m")
    }

    fn badge(&self, tone: Tone, text: &str) -> String {
        let text = printable(text);
        if !self.view.color {
            return text;
        }
        let (red, green, blue) = tone.rgb();
        format!("\x1b[1;38;2;26;20;16;48;2;{red};{green};{blue}m {text} \x1b[0m")
    }

    fn linked(&self, path: &str, text: String) -> String {
        let Some(root) = self.view.links.filter(|_| self.view.color) else {
            return text;
        };
        let mut uri = "file://".to_owned();
        for component in root.join(path).components() {
            if let Component::Normal(name) = component {
                uri.push('/');
                encode_component(&mut uri, &name.to_string_lossy());
            }
        }
        format!("\x1b]8;;{uri}\x1b\\{text}\x1b]8;;\x1b\\")
    }
}

struct Columns {
    indent: usize,
    location: usize,
    subject: usize,
    rule: usize,
    measured: usize,
}

impl Columns {
    fn of(findings: &[Finding], indent: usize) -> Self {
        let widest = |width: fn(&Finding) -> usize| findings.iter().map(width).max().unwrap_or(0);
        Self {
            indent,
            location: widest(|finding| location(finding).chars().count()),
            subject: widest(|finding| subject(finding).chars().count()),
            rule: widest(|finding| finding.rule.id().chars().count()),
            measured: widest(|finding| match &finding.detail {
                Detail::Threshold { measured, .. } => measured.to_string().len(),
                Detail::Message { .. } => 0,
            }),
        }
    }
}

pub(super) fn render(judgement: Judgement<'_>, limit: usize, view: View<'_>) -> String {
    let paint = Paint { view };
    let seal = seal(judgement);
    let Judgement {
        findings,
        context,
        unreadable,
        scanned,
    } = judgement;

    if findings.is_empty() && context.is_empty() {
        let summary = format!(
            "{}nothing to fix · {} · seal {seal}",
            scope(view),
            extent(scanned)
        );
        let mut headline = format!("{}  {}\n", mark(&paint), paint.plain(Tone::Dim, &summary));
        write_commits(&mut headline, &paint);
        return headline + &unmeasured(&paint, unreadable, limit);
    }

    let heading = format!("{}{}", scope(view), extent(scanned));
    let mut rendered = format!("{}  {}\n", mark(&paint), paint.plain(Tone::Dim, &heading));
    write_commits(&mut rendered, &paint);
    rendered.push('\n');

    let shown = &findings[..limit.min(findings.len())];
    let noted = &context[..limit.min(context.len())];
    write_files(&mut rendered, &paint, shown, noted);

    let hidden = findings.len() - shown.len() + context.len() - noted.len();
    if hidden > 0 {
        let indent = indent(view);
        let note = format!(
            "{:indent$}{hidden} more not shown. Narrow the scope with a path argument.",
            ""
        );
        writeln!(rendered, "{}\n", paint.plain(Tone::Dim, &note))
            .expect("writing to a string never fails");
    }

    writeln!(rendered, "{}", footer(&paint, judgement, &seal))
        .expect("writing to a string never fails");
    rendered.push_str(&unmeasured(&paint, unreadable, limit));

    rendered
}

struct File<'a> {
    path: &'a str,
    findings: &'a [Finding],
    context: Vec<&'a Finding>,
}

fn grouped<'a>(shown: &'a [Finding], noted: &'a [Finding]) -> Vec<File<'a>> {
    let mut files: BTreeMap<&str, File<'a>> = shown
        .chunk_by(|left, right| left.path == right.path)
        .map(|findings| {
            let path = findings[0].path.as_str();
            (
                path,
                File {
                    path,
                    findings,
                    context: Vec::new(),
                },
            )
        })
        .collect();
    for note in noted {
        let path = note.path.as_str();
        files
            .entry(path)
            .or_insert_with(|| File {
                path,
                findings: &[],
                context: Vec::new(),
            })
            .context
            .push(note);
    }

    files.into_values().collect()
}

fn write_files(rendered: &mut String, paint: &Paint<'_>, shown: &[Finding], noted: &[Finding]) {
    let mut files = grouped(shown, noted);
    let columns = Columns::of(shown, indent(paint.view));
    let rhythm = |path: &str| -> Rhythm {
        paint
            .view
            .rhythm
            .and_then(|rhythms| rhythms.get(path).copied())
            .unwrap_or_default()
    };
    if paint.view.rhythm.is_some() {
        files.sort_by_key(|file| Reverse(rhythm(file.path).commits()));
    }
    let busiest = files
        .iter()
        .flat_map(|file| rhythm(file.path).weeks)
        .max()
        .unwrap_or(0);

    for file in files {
        let path = file.path;
        let heading = paint.linked(path, paint.strong(Tone::Bone, path));
        match paint.view.rhythm {
            Some(_) => {
                let pulse = rhythm(path);
                writeln!(
                    rendered,
                    "{}  {heading}  {}",
                    spark(paint, &pulse.weeks, busiest),
                    paint.plain(Tone::Dim, &changed(pulse.commits()))
                )
            }
            None => writeln!(rendered, "{heading}"),
        }
        .expect("writing to a string never fails");
        write_rows(rendered, paint, &columns, file.findings);
        write_context(rendered, paint, columns.indent, &file.context);
        rendered.push('\n');
    }
}

fn write_context(rendered: &mut String, paint: &Paint<'_>, indent: usize, context: &[&Finding]) {
    for note in context {
        let mut text = format!("{}  ", note.rule.id());
        write_detail(&mut text, &note.detail);
        let mark = paint.plain(Tone::Amber, "◇");
        let line = paint.plain(Tone::Dim, &text);
        let row = format!("{:indent$}{mark} {line}\n", "");
        rendered.push_str(&row);
    }
}

fn write_rows(rendered: &mut String, paint: &Paint<'_>, columns: &Columns, file: &[Finding]) {
    let mut previous: Option<&Finding> = None;
    for (index, finding) in file.iter().enumerate() {
        let repeated = previous.is_some_and(|before| same_unit(before, finding));
        write_row(rendered, paint, columns, finding, repeated);
        previous = Some(finding);

        let next = file.get(index + 1);
        if next.is_none_or(|after| after.span.start_line != finding.span.start_line) {
            write_trail_of(rendered, paint, columns.indent + 3, finding, next.is_some());
        }
    }
}

fn write_trail_of(
    rendered: &mut String,
    paint: &Paint<'_>,
    indent: usize,
    finding: &Finding,
    followed: bool,
) {
    let key = (finding.path.clone(), finding.span.start_line);
    let Some(lines) = paint.view.trail.and_then(|trail| trail.excerpts.get(&key)) else {
        return;
    };
    if lines.is_empty() {
        return;
    }
    write_excerpt(rendered, paint, indent, lines);
    if followed {
        rendered.push('\n');
    }
}

fn write_excerpt(rendered: &mut String, paint: &Paint<'_>, indent: usize, lines: &[Line]) {
    for line in lines {
        let (gutter, tone) = if line.changed {
            (paint.plain(Tone::Moss, "┃"), Tone::Bone)
        } else {
            (paint.plain(Tone::Dim, "│"), Tone::Dim)
        };
        let number = format!("{:>4}", line.number);
        let text = clipped(line.text.trim_end());
        let code = if text.is_empty() {
            String::new()
        } else {
            format!(" {}", paint.plain(tone, &text))
        };
        writeln!(
            rendered,
            "{:indent$}{} {gutter}{code}",
            "",
            paint.plain(Tone::Dim, &number),
        )
        .expect("writing to a string never fails");
    }
}

fn write_commits(rendered: &mut String, paint: &Paint<'_>) {
    let commits = paint.view.trail.map_or(&[][..], |trail| &trail.commits);
    if commits.is_empty() {
        return;
    }
    for (index, commit) in commits.iter().take(COMMITS_SHOWN).enumerate() {
        let branch = if index == 0 { "  ┬ " } else { "  │ " };
        writeln!(
            rendered,
            "{}{}  {}",
            paint.plain(Tone::Dim, branch),
            paint.plain(Tone::Amber, &commit.hash),
            paint.plain(Tone::Dim, &clipped(&commit.subject)),
        )
        .expect("writing to a string never fails");
    }
    if let Some(note) = more_commits(commits.len()) {
        writeln!(rendered, "{}", paint.plain(Tone::Dim, &note))
            .expect("writing to a string never fails");
    }
    writeln!(rendered, "{}", paint.plain(Tone::Dim, "  ┴"))
        .expect("writing to a string never fails");
}

fn more_commits(count: usize) -> Option<String> {
    match count.saturating_sub(COMMITS_SHOWN) {
        0 => None,
        1 => Some("  │ 1 more commit".to_owned()),
        hidden => Some(format!("  │ {hidden} more commits")),
    }
}

fn clipped(text: &str) -> String {
    let expanded = text.replace('\t', "    ");
    if expanded.chars().count() <= CODE_WIDTH {
        return expanded;
    }
    let kept: String = expanded.chars().take(CODE_WIDTH - 1).collect();
    format!("{kept}…")
}

fn indent(view: View<'_>) -> usize {
    view.rhythm.map_or(2, |_| WEEKS + 2)
}

fn spark(paint: &Paint<'_>, weeks: &[u32; WEEKS], busiest: u32) -> String {
    weeks
        .iter()
        .map(|&commits| {
            if commits == 0 {
                return paint.plain(Tone::Dim, SPARK[0]);
            }
            let level = (commits * 7).div_ceil(busiest) as usize;
            paint.plain(Tone::Amber, SPARK[level])
        })
        .collect()
}

fn changed(commits: u32) -> String {
    format!("{} in {WEEKS} weeks", plural(commits as usize, "commit"))
}

fn write_row(
    rendered: &mut String,
    paint: &Paint<'_>,
    columns: &Columns,
    finding: &Finding,
    repeated: bool,
) {
    let (place, name) = if repeated {
        (String::new(), String::new())
    } else {
        (location(finding), subject(finding))
    };
    let place = format!("{place:<width$}", width = columns.location);
    let name = format!("{name:<width$}", width = columns.subject);
    let rule = format!("{:<width$}", finding.rule.id(), width = columns.rule);

    write!(
        rendered,
        "{:indent$}{}  {}  {}  ",
        "",
        paint.plain(Tone::Dim, &place),
        paint.plain(Tone::Amber, &name),
        paint.plain(Tone::Dim, &rule),
        indent = columns.indent,
    )
    .expect("writing to a string never fails");

    let tone = Tone::Scale;
    match &finding.detail {
        Detail::Threshold { measured, limit } => {
            let value = format!("{measured:>width$}", width = columns.measured);
            write!(
                rendered,
                "{}  {}{}",
                bar(paint, tone, *measured, *limit),
                paint.strong(tone, &value),
                paint.plain(Tone::Dim, &format!("/{limit}")),
            )
            .expect("writing to a string never fails");
        }
        Detail::Message { message } => rendered.push_str(&paint.plain(tone, message)),
    }
    rendered.push('\n');
}

fn bar(paint: &Paint<'_>, tone: Tone, measured: u32, limit: u32) -> String {
    let top = measured.max(limit);
    let scale = top + top / 6;
    let mark = limit * BAR_WIDTH / scale;
    let reach = measured * BAR_WIDTH / scale;
    let over = reach.saturating_sub(mark).max(1);
    let rest = BAR_WIDTH.saturating_sub(mark + over);

    let cells = |glyph: &str, width: u32| glyph.repeat(width as usize);
    format!(
        "{}{}{}{}",
        paint.plain(Tone::Dim, &cells("━", mark)),
        paint.plain(Tone::Amber, "┃"),
        paint.plain(tone, &cells("━", over)),
        paint.plain(Tone::Dim, &cells("╌", rest)),
    )
}

fn footer(paint: &Paint<'_>, judgement: Judgement<'_>, seal: &str) -> String {
    let Judgement {
        findings,
        context,
        unreadable,
        ..
    } = judgement;
    let mut notes = Vec::new();

    let badge = if findings.is_empty() {
        paint.badge(Tone::Moss, "nothing to fix")
    } else {
        paint.badge(Tone::Scale, &format!("{} to fix", findings.len()))
    };
    if !context.is_empty() {
        notes.push(plural(context.len(), "note"));
    }
    if !unreadable.is_empty() {
        notes.push(format!("{} unreadable", unreadable.len()));
    }
    notes.push(format!("seal {seal}"));

    let separator = if paint.view.color { "  " } else { " · " };
    format!(
        "{badge}{separator}{}",
        paint.plain(Tone::Dim, &notes.join(" · "))
    )
}

fn unmeasured(paint: &Paint<'_>, unreadable: &[Unreadable], limit: usize) -> String {
    let mut rendered = String::new();
    for line in super::unreadable(unreadable, limit).lines() {
        writeln!(rendered, "{}", paint.plain(Tone::Dim, line))
            .expect("writing to a string never fails");
    }
    rendered
}

fn mark(paint: &Paint<'_>) -> String {
    format!(
        "{} {}",
        paint.strong(Tone::Amber, "⬢"),
        paint.strong(Tone::Bone, "jabuti")
    )
}

fn scope(view: View<'_>) -> String {
    view.since
        .map_or_else(String::new, |reference| format!("since {reference} · "))
}

fn extent(scanned: Scanned) -> String {
    format!(
        "{} · {}",
        plural(scanned.files, "file"),
        plural(scanned.units, "unit")
    )
}

fn location(finding: &Finding) -> String {
    format!(":{}", finding.span.start_line)
}

fn subject(finding: &Finding) -> String {
    let name = finding.subject.as_deref().unwrap_or("");
    if name.chars().count() <= SUBJECT_WIDTH {
        return name.to_owned();
    }
    let kept: String = name.chars().take(SUBJECT_WIDTH - 1).collect();
    format!("{kept}…")
}

fn same_unit(left: &Finding, right: &Finding) -> bool {
    left.subject.is_some()
        && left.subject == right.subject
        && left.span.start_line == right.span.start_line
}

fn reorders(character: char) -> bool {
    matches!(character, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

fn printable(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() || reorders(character) {
                char::REPLACEMENT_CHARACTER
            } else {
                character
            }
        })
        .collect()
}

fn seal(judgement: Judgement<'_>) -> String {
    let digest = json(judgement).bytes().fold(FNV_OFFSET, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    });
    let high = digest >> 48;
    let low = (digest >> 32) & 0xffff;
    format!("{high:04x}·{low:04x}")
}
