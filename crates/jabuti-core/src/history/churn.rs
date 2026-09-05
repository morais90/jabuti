use std::collections::BTreeMap;
use std::path::PathBuf;

pub fn tally(log: &str) -> BTreeMap<PathBuf, u32> {
    let mut commits: BTreeMap<PathBuf, u32> = BTreeMap::new();

    for line in log.lines() {
        if let Some(path) = touched_path(line) {
            *commits.entry(path).or_default() += 1;
        }
    }

    commits
}

fn touched_path(line: &str) -> Option<PathBuf> {
    let mut columns = line.split('\t');
    let _added = columns.next()?;
    let _removed = columns.next()?;

    columns.next().map(PathBuf::from)
}
