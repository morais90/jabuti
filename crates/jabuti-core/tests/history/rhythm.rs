use std::collections::BTreeMap;

use jabuti_core::history::rhythm;
use jabuti_core::report::Rhythm;

const HEAD: i64 = 1_790_000_000;
const WEEK: i64 = 7 * 24 * 60 * 60;

fn commit(age: i64, paths: &[&str]) -> String {
    format!("\u{1}{}\0\n{}\0", HEAD - age, paths.join("\0"))
}

#[test]
fn each_commit_lands_in_the_week_it_was_written_counting_back_from_head() {
    let log = [
        commit(0, &["src/busy.rs", "src/quiet.rs"]),
        commit(WEEK - 1, &["src/busy.rs"]),
        commit(WEEK, &["src/busy.rs"]),
        commit(11 * WEEK + WEEK - 1, &["src/busy.rs"]),
    ]
    .concat();

    assert_eq!(
        rhythm::read(&log, HEAD).expect("the log is well formed"),
        BTreeMap::from([
            (
                "src/busy.rs".to_owned(),
                Rhythm {
                    weeks: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2],
                },
            ),
            (
                "src/quiet.rs".to_owned(),
                Rhythm {
                    weeks: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
                },
            ),
        ])
    );
}

#[test]
fn a_commit_written_before_the_weeks_drawn_is_not_counted() {
    let log = [
        commit(12 * WEEK, &["src/old.rs"]),
        commit(0, &["src/new.rs"]),
    ]
    .concat();

    assert_eq!(
        rhythm::read(&log, HEAD)
            .expect("the log is well formed")
            .keys()
            .collect::<Vec<_>>(),
        ["src/new.rs"]
    );
}

#[test]
fn a_commit_dated_after_head_belongs_to_the_latest_week() {
    let log = commit(-WEEK, &["src/skewed.rs"]);

    assert_eq!(
        rhythm::read(&log, HEAD).expect("the log is well formed")["src/skewed.rs"].weeks,
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
    );
}

#[test]
fn a_name_git_would_quote_is_read_exactly_as_written() {
    let log = commit(0, &["src/a\"b\\c.rs", "app/[id]/page.ts"]);

    assert_eq!(
        rhythm::read(&log, HEAD)
            .expect("the log is well formed")
            .keys()
            .collect::<Vec<_>>(),
        ["app/[id]/page.ts", "src/a\"b\\c.rs"]
    );
}

#[test]
fn the_window_opens_twelve_weeks_before_head() {
    assert_eq!(rhythm::window(HEAD), HEAD - 12 * WEEK);
}

#[test]
fn a_rhythm_counts_the_commits_of_every_week() {
    let rhythm = Rhythm {
        weeks: [1, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 2],
    };

    assert_eq!(rhythm.commits(), 6);
}

#[test]
fn a_commit_time_that_is_not_a_number_is_refused() {
    let log = "\u{1}soon\0\nsrc/a.rs\0";

    assert_eq!(
        rhythm::read(log, HEAD),
        Err(rhythm::MalformedLog("soon".to_owned()))
    );
}

#[test]
fn a_head_far_in_the_past_opens_the_window_at_the_earliest_time() {
    assert_eq!(rhythm::window(i64::MIN + 1), i64::MIN);
}

#[test]
fn a_commit_far_before_head_is_left_out_rather_than_overflowing() {
    let log = format!("\u{1}{}\0\nsrc/a.rs\0", i64::MIN);

    assert!(
        rhythm::read(&log, i64::MAX)
            .expect("the log is well formed")
            .is_empty()
    );
}
