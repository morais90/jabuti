use std::collections::BTreeMap;

use crate::report::{Rhythm, WEEKS};

const WEEK_SECONDS: i64 = 7 * 24 * 60 * 60;
const COMMIT: char = '\u{1}';

pub const FORMAT: &str = "--format=%x01%at";

pub fn window(head: i64) -> i64 {
    let weeks = i64::try_from(WEEKS).expect("the weeks drawn fit in an i64");
    head - WEEK_SECONDS * weeks
}

pub fn read(log: &str, head: i64) -> BTreeMap<String, Rhythm> {
    let mut rhythms: BTreeMap<String, Rhythm> = BTreeMap::new();
    let mut week = None;

    for token in log.split('\0').filter(|token| !token.is_empty()) {
        if let Some(stamp) = token.strip_prefix(COMMIT) {
            week = stamp
                .parse::<i64>()
                .ok()
                .and_then(|time| week_of(head, time));
            continue;
        }
        let Some(index) = week else {
            continue;
        };
        let path = token.strip_prefix('\n').unwrap_or(token);
        rhythms.entry(path.to_owned()).or_default().weeks[index] += 1;
    }

    rhythms
}

fn week_of(head: i64, time: i64) -> Option<usize> {
    let age = (head - time).max(0) / WEEK_SECONDS;
    let age = usize::try_from(age).ok()?;
    WEEKS.checked_sub(age + 1)
}
