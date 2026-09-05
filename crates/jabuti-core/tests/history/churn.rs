use std::collections::BTreeMap;
use std::path::PathBuf;

use jabuti_core::history::churn;

#[test]
fn every_numstat_line_counts_one_commit_against_its_path() {
    let log = "3\t1\tsrc/busy.rs\n\n1\t0\tsrc/busy.rs\n0\t2\tsrc/quiet.rs\n-\t-\tassets/logo.png\n";

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
fn a_line_without_the_three_numstat_columns_counts_for_nothing() {
    assert_eq!(churn::tally("commit abc\n\n3\t1\n"), BTreeMap::new());
}
