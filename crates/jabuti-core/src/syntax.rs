use tree_sitter::{Node, Parser, Query, QueryCursor, QueryMatch, StreamingIterator, Tree};

use crate::lang::{LangSpec, LanguageId};
use crate::model::Span;

#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    #[error("parser rejected the grammar: {0}")]
    Grammar(#[from] tree_sitter::LanguageError),
    #[error("unreadable syntax from line {line}")]
    Malformed { line: u32 },
}

#[derive(Debug)]
pub struct Parsed<'source> {
    tree: Tree,
    source: &'source str,
    spec: &'static LangSpec,
}

pub fn parse<'source>(
    source: &'source str,
    spec: &'static LangSpec,
) -> Result<Parsed<'source>, SyntaxError> {
    let mut parser = Parser::new();
    parser.set_language(spec.language())?;

    let tree = parser
        .parse(source, None)
        .ok_or(SyntaxError::Malformed { line: 1 })?;
    if let Some(line) = first_error(tree.root_node()) {
        return Err(SyntaxError::Malformed { line });
    }

    Ok(Parsed { tree, source, spec })
}

impl<'source> Parsed<'source> {
    pub fn language(&self) -> LanguageId {
        self.spec.id
    }

    pub(crate) fn source(&self) -> &'source str {
        self.source
    }

    pub(crate) fn root(&self) -> Node<'_> {
        self.tree.root_node()
    }

    pub(crate) fn for_each_match(
        &self,
        query: &Query,
        mut visit: impl FnMut(&QueryMatch<'_, '_>, &Query),
    ) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());

        while let Some(matched) = matches.next() {
            visit(matched, query);
        }
    }
}

pub(crate) fn text_of(node: Node<'_>, source: &str) -> String {
    node.utf8_text(source.as_bytes())
        .unwrap_or_default()
        .to_owned()
}

fn first_error(node: Node<'_>) -> Option<u32> {
    if !node.has_error() {
        return None;
    }

    let mut cursor = node.walk();
    let inside = node.children(&mut cursor).find_map(first_error);

    inside.or_else(|| {
        (node.is_error() || node.is_missing()).then(|| line_number(node.start_position().row))
    })
}

pub(crate) fn span_of(node: Node<'_>) -> Span {
    Span {
        start_line: line_number(node.start_position().row),
        end_line: line_number(node.end_position().row),
    }
}

pub(crate) fn line_number(row: usize) -> u32 {
    u32::try_from(row.saturating_add(1)).unwrap_or(u32::MAX)
}
