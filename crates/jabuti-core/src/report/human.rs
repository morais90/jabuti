use std::ffi::OsStr;
use std::fmt::Write;
use std::path::{Component, Path};

use super::sarif::encode_component;
use super::{Scanned, count, json, plural};
use crate::catalog::Severity;
use crate::model::{Detail, Finding, Unreadable};

const BAR_WIDTH: u32 = 12;
const SUBJECT_WIDTH: usize = 24;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug, Clone, Copy)]
pub struct View<'a> {
    pub since: Option<&'a str>,
    pub color: bool,
    pub links: Option<&'a Path>,
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
}

impl Tone {
    fn rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Amber => (0xe8, 0xb3, 0x3a),
            Self::Scale => (0xe2, 0x55, 0x2a),
            Self::Bone => (0xd9, 0xcf, 0xbd),
            Self::Dim => (0x7d, 0x73, 0x66),
        }
    }

    fn of(severity: Severity) -> Self {
        match severity {
            Severity::Error => Self::Scale,
            Severity::Warning => Self::Amber,
            Severity::Off => Self::Dim,
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
    location: usize,
    subject: usize,
    rule: usize,
    measured: usize,
}

impl Columns {
    fn of(findings: &[Finding]) -> Self {
        let widest = |width: fn(&Finding) -> usize| findings.iter().map(width).max().unwrap_or(0);
        Self {
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

pub(super) fn render(
    findings: &[Finding],
    unreadable: &[Unreadable],
    scanned: Scanned,
    limit: usize,
    view: View<'_>,
) -> String {
    let paint = Paint { view };
    let seal = seal(findings, unreadable, scanned);

    if findings.is_empty() {
        let summary = format!(
            "{}nothing to fix · {} · seal {seal}",
            scope(view),
            extent(scanned)
        );
        let headline = format!("{}  {}\n", mark(&paint), paint.plain(Tone::Dim, &summary));
        return headline + &unmeasured(&paint, unreadable, limit);
    }

    let heading = format!("{}{}", scope(view), extent(scanned));
    let mut rendered = format!("{}  {}\n\n", mark(&paint), paint.plain(Tone::Dim, &heading));

    let shown = &findings[..limit.min(findings.len())];
    write_files(&mut rendered, &paint, shown);

    let hidden = findings.len() - shown.len();
    if hidden > 0 {
        let note = format!("  {hidden} more not shown. Narrow the scope with a path argument.");
        writeln!(rendered, "{}\n", paint.plain(Tone::Dim, &note))
            .expect("writing to a string never fails");
    }

    writeln!(rendered, "{}", footer(&paint, findings, unreadable, &seal))
        .expect("writing to a string never fails");
    rendered.push_str(&unmeasured(&paint, unreadable, limit));

    rendered
}

fn write_files(rendered: &mut String, paint: &Paint<'_>, shown: &[Finding]) {
    let columns = Columns::of(shown);
    for file in shown.chunk_by(|left, right| left.path == right.path) {
        let path = paint.strong(Tone::Bone, &file[0].path);
        writeln!(rendered, "{}", paint.linked(&file[0].path, path))
            .expect("writing to a string never fails");
        let mut previous: Option<&Finding> = None;
        for finding in file {
            let repeated = previous.is_some_and(|before| same_unit(before, finding));
            write_row(rendered, paint, &columns, finding, repeated);
            previous = Some(finding);
        }
        rendered.push('\n');
    }
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
        "  {}  {}  {}  ",
        paint.plain(Tone::Dim, &place),
        paint.plain(Tone::Amber, &name),
        paint.plain(Tone::Dim, &rule),
    )
    .expect("writing to a string never fails");

    let tone = Tone::of(finding.severity);
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

fn footer(
    paint: &Paint<'_>,
    findings: &[Finding],
    unreadable: &[Unreadable],
    seal: &str,
) -> String {
    let errors = count(findings, Severity::Error);
    let warnings = count(findings, Severity::Warning);
    let mut notes = Vec::new();

    let badge = if errors > 0 {
        if warnings > 0 {
            notes.push(plural(warnings, "warning"));
        }
        paint.badge(Tone::Scale, &format!("{errors} to fix"))
    } else {
        paint.badge(Tone::Amber, &plural(warnings, "warning"))
    };
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

fn printable(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() {
                char::REPLACEMENT_CHARACTER
            } else {
                character
            }
        })
        .collect()
}

fn seal(findings: &[Finding], unreadable: &[Unreadable], scanned: Scanned) -> String {
    let digest = json(findings, unreadable, scanned)
        .bytes()
        .fold(FNV_OFFSET, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
        });
    let high = digest >> 48;
    let low = (digest >> 32) & 0xffff;
    format!("{high:04x}·{low:04x}")
}
