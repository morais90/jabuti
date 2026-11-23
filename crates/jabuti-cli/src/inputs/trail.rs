use std::path::Path;

use jabuti_core::model::Span;
use jabuti_core::report::trail::{self, Trail};

use crate::inputs::git::since::Changes;

pub(crate) fn load(root: &Path, units: &[(&str, u32)], changes: Option<&Changes>) -> Trail {
    let touched = |path: &str, line: u32| {
        let span = Span {
            start_line: line,
            end_line: line,
        };
        changes.is_some_and(|changes| changes.touches(Path::new(path), span))
    };

    let excerpts = units
        .iter()
        .filter_map(|&(path, line)| {
            let text = std::fs::read_to_string(root.join(path)).ok()?;
            let lines = trail::excerpt(&text, line, |number| touched(path, number));
            Some(((path.to_owned(), line), lines))
        })
        .collect();
    let commits = changes
        .and_then(|changes| changes.commits().ok())
        .map_or_else(Vec::new, |log| trail::commits(&log));

    Trail { excerpts, commits }
}
