use std::ops::Range;

use tree_sitter::{Node, Parser, Query, QueryCursor, QueryMatch, StreamingIterator, Tree};

use crate::lang::LangSpec;
use crate::model::{Span, Unit, UnitKind};

#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    #[error("parser rejected the grammar: {0}")]
    Grammar(#[from] tree_sitter::LanguageError),
    #[error("query failed to compile: {0}")]
    Query(#[from] tree_sitter::QueryError),
    #[error("source could not be parsed")]
    Malformed,
}

pub struct Parsed<'source> {
    tree: Tree,
    source: &'source str,
    spec: &'static LangSpec,
    units_query: Query,
}

pub fn parse<'source>(
    source: &'source str,
    spec: &'static LangSpec,
) -> Result<Parsed<'source>, SyntaxError> {
    let language = spec.language();
    let mut parser = Parser::new();
    parser.set_language(&language)?;

    let tree = parser.parse(source, None).ok_or(SyntaxError::Malformed)?;
    if tree.root_node().has_error() {
        return Err(SyntaxError::Malformed);
    }

    let units_query = Query::new(&language, spec.units_query)?;

    Ok(Parsed {
        tree,
        source,
        spec,
        units_query,
    })
}

impl<'source> Parsed<'source> {
    pub fn source(&self) -> &'source str {
        self.source
    }

    pub fn spec(&self) -> &'static LangSpec {
        self.spec
    }

    pub fn units(&self) -> Unit {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(
            &self.units_query,
            self.tree.root_node(),
            self.source.as_bytes(),
        );

        let mut captured = Vec::new();
        while let Some(matched) = matches.next() {
            if let Some(unit) = CapturedUnit::from_match(matched, &self.units_query, self.source) {
                captured.push(unit);
            }
        }

        nest(captured, span_of(self.tree.root_node()))
    }
}

struct CapturedUnit {
    kind: UnitKind,
    name: Option<String>,
    span: Span,
    bytes: Range<usize>,
}

impl CapturedUnit {
    fn from_match(matched: &QueryMatch, query: &Query, source: &str) -> Option<Self> {
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
    match label.strip_prefix("unit.")? {
        "module" => Some(UnitKind::Module),
        "type" => Some(UnitKind::Type),
        "function" => Some(UnitKind::Function),
        "closure" => Some(UnitKind::Closure),
        _ => None,
    }
}

fn span_of(node: Node) -> Span {
    let start = node.start_position();
    let end = node.end_position();
    let last_row = if end.column == 0 && end.row > start.row {
        end.row - 1
    } else {
        end.row
    };

    Span {
        start_line: start.row as u32 + 1,
        end_line: last_row as u32 + 1,
    }
}

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
