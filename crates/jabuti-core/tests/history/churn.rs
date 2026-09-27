use std::collections::BTreeMap;
use std::path::PathBuf;

use jabuti_core::history::churn;

#[test]
fn every_path_a_commit_lists_counts_that_commit_against_it() {
    let log = "\nsrc/busy.rs\nsrc/quiet.rs\n\nsrc/busy.rs\n\nassets/logo.png\n";

    assert_eq!(
        churn::tally(log),
        BTreeMap::from([
            (PathBuf::from("assets/logo.png"), 1),
            (PathBuf::from("src/busy.rs"), 2),
            (PathBuf::from("src/quiet.rs"), 1),
        ])
    );
}

#[test]
fn the_blank_lines_between_commits_count_for_nothing() {
    assert_eq!(churn::tally("\n\n\n"), BTreeMap::new());
}
