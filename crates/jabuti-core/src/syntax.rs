use std::ops::Range;

use tree_sitter::{Node, Parser, Query, QueryCursor, QueryMatch, StreamingIterator, Tree};

use crate::lang::{LangSpec, Queries};
use crate::model::{Span, Unit, UnitKind};

#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    #[error("parser rejected the grammar: {0}")]
    Grammar(#[from] tree_sitter::LanguageError),
    #[error("source could not be parsed")]
    Malformed,
}

#[derive(Debug)]
pub struct Parsed<'source> {
    tree: Tree,
    source: &'source str,
    queries: &'static Queries,
}

pub fn parse<'source>(
    source: &'source str,
    spec: &'static LangSpec,
) -> Result<Parsed<'source>, SyntaxError> {
    let queries = spec.queries();
    let mut parser = Parser::new();
    parser.set_language(&queries.language)?;

    let tree = parser.parse(source, None).ok_or(SyntaxError::Malformed)?;
    if tree.root_node().has_error() {
        return Err(SyntaxError::Malformed);
    }

    Ok(Parsed {
        tree,
        source,
        queries,
    })
}

impl Parsed<'_> {
    pub fn units(&self) -> Unit {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(
            &self.queries.units,
            self.tree.root_node(),
            self.source.as_bytes(),
        );

        let mut captured = Vec::new();
        while let Some(matched) = matches.next() {
            if let Some(unit) = CapturedUnit::from_match(matched, &self.queries.units, self.source)
            {
                captured.push(unit);
            }
        }

        nest(captured, file_span(self.source))
    }

    pub fn comment_ranges(&self) -> Vec<Range<usize>> {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(
            &self.queries.comments,
            self.tree.root_node(),
            self.source.as_bytes(),
        );

        let mut ranges = Vec::new();
        while let Some(matched) = matches.next() {
            for capture in matched.captures {
                ranges.push(capture.node.byte_range());
            }
        }

        ranges.sort_by_key(|range| range.start);
        ranges
    }
}

fn file_span(source: &str) -> Span {
    let lines = source.split_inclusive('\n').count().max(1);

    Span {
        start_line: 1,
        end_line: u32::try_from(lines).unwrap_or(u32::MAX),
    }
}

#[derive(Debug)]
struct CapturedUnit {
    kind: UnitKind,
    name: Option<String>,
    span: Span,
    bytes: Range<usize>,
}

impl CapturedUnit {
    fn from_match(matched: &QueryMatch<'_, '_>, query: &Query, source: &str) -> Option<Self> {
        let mut kind = None;
        let mut node = None;
        let mut name = None;

        for capture in matched.captures {
            let label = query.capture_names()[capture.index as usize];
            if let Some(labelled_kind) = kind_of_label(label) {
                kind = Some(labelled_kind);
                node = Some(capture.node);
            } else if label == "name" {
                name = capture
                    .node
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(str::to_owned);
            }
        }

        let node = node?;
        Some(Self {
            kind: kind?,
            name,
            span: span_of(node),
            bytes: node.byte_range(),
        })
    }
}

fn kind_of_label(label: &str) -> Option<UnitKind> {
    let suffix = label.strip_prefix("unit.")?;

    match suffix {
        "module" => Some(UnitKind::Module),
        "type" => Some(UnitKind::Type),
        "function" => Some(UnitKind::Function),
        "closure" => Some(UnitKind::Closure),
        unknown => panic!("query captures @unit.{unknown}, which maps to no unit kind"),
    }
}

fn span_of(node: Node<'_>) -> Span {
    Span {
        start_line: line_number(node.start_position().row),
        end_line: line_number(node.end_position().row),
    }
}

fn line_number(row: usize) -> u32 {
    u32::try_from(row.saturating_add(1)).unwrap_or(u32::MAX)
}

#[derive(Debug)]
struct OpenUnit {
    unit: Unit,
    bytes: Range<usize>,
}

fn nest(mut captured: Vec<CapturedUnit>, file_span: Span) -> Unit {
    captured.sort_by(|left, right| {
        left.bytes
            .start
            .cmp(&right.bytes.start)
            .then(right.bytes.end.cmp(&left.bytes.end))
    });

    let mut file = Unit {
        name: None,
        kind: UnitKind::File,
        span: file_span,
        children: Vec::new(),
    };
    let mut open: Vec<OpenUnit> = Vec::new();

    for capture in captured {
        while let Some(closed) = close_enclosing(&mut open, &capture.bytes) {
            attach(&mut open, &mut file, closed);
        }
        open.push(OpenUnit {
            unit: Unit {
                name: capture.name,
                kind: capture.kind,
                span: capture.span,
                children: Vec::new(),
            },
            bytes: capture.bytes,
        });
    }

    while let Some(remaining) = open.pop() {
        attach(&mut open, &mut file, remaining.unit);
    }

    file
}

fn close_enclosing(open: &mut Vec<OpenUnit>, bytes: &Range<usize>) -> Option<Unit> {
    let innermost = open.last()?;
    if innermost.bytes.start <= bytes.start && bytes.end <= innermost.bytes.end {
        return None;
    }
    open.pop().map(|closed| closed.unit)
}

fn attach(open: &mut [OpenUnit], file: &mut Unit, unit: Unit) {
    match open.last_mut() {
        Some(parent) => parent.unit.children.push(unit),
        None => file.children.push(unit),
    }
}
