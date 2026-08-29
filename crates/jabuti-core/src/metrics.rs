use std::cmp::Ordering;
use std::ops::Range;

use crate::model::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Loc {
    pub total: u32,
    pub code: u32,
    pub comment: u32,
    pub blank: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineKind {
    Blank,
    Comment,
    Code,
}

#[derive(Debug)]
pub struct LineIndex {
    kinds: Vec<LineKind>,
}

impl LineIndex {
    pub fn new(source: &str, comments: &[Range<usize>]) -> Self {
        let mut ordered = comments.to_vec();
        ordered.sort_by_key(|comment| comment.start);

        let mut kinds = Vec::new();
        let mut line_start = 0;

        for line in source.split_inclusive('\n') {
            kinds.push(classify(line, line_start, &ordered));
            line_start += line.len();
        }

        Self { kinds }
    }

    pub fn loc(&self, span: Span) -> Loc {
        let first = span.start_line.saturating_sub(1) as usize;
        let last = (span.end_line as usize).min(self.kinds.len());
        let counted = self.kinds.get(first..last).unwrap_or_default();

        let mut loc = Loc {
            total: u32::try_from(counted.len()).unwrap_or(u32::MAX),
            code: 0,
            comment: 0,
            blank: 0,
        };

        for kind in counted {
            match kind {
                LineKind::Blank => loc.blank += 1,
                LineKind::Comment => loc.comment += 1,
                LineKind::Code => loc.code += 1,
            }
        }

        loc
    }
}

fn classify(line: &str, line_start: usize, comments: &[Range<usize>]) -> LineKind {
    let mut occupied = false;

    for (offset, character) in line.char_indices() {
        if character.is_whitespace() {
            continue;
        }

        occupied = true;
        if !is_commented(line_start + offset, comments) {
            return LineKind::Code;
        }
    }

    if occupied {
        LineKind::Comment
    } else {
        LineKind::Blank
    }
}

fn is_commented(position: usize, comments: &[Range<usize>]) -> bool {
    comments
        .binary_search_by(|comment| {
            if comment.end <= position {
                Ordering::Less
            } else if comment.start > position {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
        .is_ok()
}
