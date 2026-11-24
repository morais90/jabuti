use std::collections::BTreeMap;

use crate::report::{Rhythm, WEEKS};

const WEEK_SECONDS: i64 = 7 * 24 * 60 * 60;
const COMMIT: char = '\u{1}';

pub const FORMAT: &str = "--format=%x01%at";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("git gave a commit time that is not a number: {0}")]
pub struct MalformedLog(pub String);

pub fn window(head: i64) -> i64 {
    let weeks = i64::try_from(WEEKS).unwrap_or(i64::MAX);
    head.saturating_sub(WEEK_SECONDS.saturating_mul(weeks))
}

pub fn read(log: &str, head: i64) -> Result<BTreeMap<String, Rhythm>, MalformedLog> {
    let mut rhythms: BTreeMap<String, Rhythm> = BTreeMap::new();
    let mut week = None;

    for token in log.split('\0').filter(|token| !token.is_empty()) {
        if let Some(stamp) = token.strip_prefix(COMMIT) {
            let time = stamp
                .parse::<i64>()
                .map_err(|_| MalformedLog(stamp.to_owned()))?;
            week = week_of(head, time);
            continue;
        }
        let Some(index) = week else {
            continue;
        };
        let path = token.strip_prefix('\n').unwrap_or(token);
        rhythms.entry(path.to_owned()).or_default().weeks[index] += 1;
    }

    Ok(rhythms)
}

fn week_of(head: i64, time: i64) -> Option<usize> {
    let age = head.saturating_sub(time).max(0) / WEEK_SECONDS;
    (0..WEEKS)
        .rev()
        .zip(0_i64..)
        .find_map(|(index, weeks_ago)| (weeks_ago == age).then_some(index))
}
