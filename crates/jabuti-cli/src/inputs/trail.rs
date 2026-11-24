use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use jabuti_core::model::Span;
use jabuti_core::report::trail::{self, Trail};

use crate::inputs::git::since::Changes;

pub(crate) fn load(root: &Path, units: &[(&str, u32)], changes: Option<&Changes>) -> Result<Trail> {
    let touched = |path: &str, line: u32| {
        let span = Span {
            start_line: line,
            end_line: line,
        };
        changes.is_some_and(|changes| changes.touches(Path::new(path), span))
    };

    let mut excerpts = BTreeMap::new();
    let shown = units.iter().filter(|(path, _)| root.join(path).is_file());
    for &(path, line) in shown {
        let text =
            fs::read_to_string(root.join(path)).with_context(|| format!("reading {path}"))?;
        let lines = trail::excerpt(&text, line, |number| touched(path, number));
        excerpts.insert((path.to_owned(), line), lines);
    }
    let log = changes.map(Changes::commits).transpose()?;
    let commits = log.map_or_else(Vec::new, |log| trail::commits(&log));

    Ok(Trail { excerpts, commits })
}
