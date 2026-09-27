use std::collections::BTreeMap;
use std::path::PathBuf;

pub fn tally(log: &str) -> BTreeMap<PathBuf, u32> {
    let mut commits: BTreeMap<PathBuf, u32> = BTreeMap::new();

    for path in log.lines().filter(|line| !line.is_empty()) {
        *commits.entry(PathBuf::from(path)).or_default() += 1;
    }

    commits
}
