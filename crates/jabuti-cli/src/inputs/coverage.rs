use std::fs;
use std::path::Path;
use std::time::SystemTime;

use jabuti_core::tools::coverage::{Coverage, Format};

use super::workspace;

pub(crate) fn read(
    report: &Path,
    project: &Path,
    sources: &[(&Path, &str)],
) -> Result<Coverage, String> {
    let shown = workspace::display(report, project);
    let Some(format) = Format::of(report) else {
        return Err(format!("{shown} has an extension jabuti cannot read"));
    };
    let written = modified(report).map_err(|error| format!("{shown} {error}"))?;

    for (path, source) in sources {
        let changed = modified(path).map_err(|error| format!("{source} {error}"))?;
        if written < changed {
            return Err(format!("{shown} is older than {source}"));
        }
    }

    let text = fs::read_to_string(report).map_err(|error| format!("{shown} {error}"))?;

    Coverage::parse(format, &text).map_err(|error| format!("{shown} {error}"))
}

fn modified(path: &Path) -> std::io::Result<SystemTime> {
    fs::metadata(path)?.modified()
}
