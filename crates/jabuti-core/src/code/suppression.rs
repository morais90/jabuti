use super::concepts::Occurrence;
use super::lang;
use crate::catalog::{Rule, RuleId};
use crate::lang::LanguageId;
use crate::model::{Detail, Finding};
use crate::policy::Policy;

pub fn findings(
    path: &str,
    language: LanguageId,
    occurrences: &[Occurrence],
    policy: &Policy,
) -> Vec<Finding> {
    let Some(config) = policy.active_for(language, Rule::Suppression) else {
        return Vec::new();
    };

    let concepts = Rule::Suppression.spec().concepts;

    occurrences
        .iter()
        .filter(|occurrence| concepts.contains(&occurrence.concept))
        .map(|occurrence| {
            let subject = subject_of(&occurrence.subject);
            Finding {
                rule: RuleId::Native(Rule::Suppression),
                severity: config.severity,
                path: path.to_owned(),
                span: occurrence.span,
                subject: Some(subject.clone()),
                detail: Detail::Message {
                    message: consequence(language, &subject).to_owned(),
                },
            }
        })
        .collect()
}

fn consequence(language: LanguageId, subject: &str) -> &'static str {
    if lang::table(language).type_escapes.contains(&subject) {
        "a type-checker diagnostic is suppressed instead of satisfied"
    } else {
        "a linter diagnostic is suppressed instead of satisfied"
    }
}

fn subject_of(raw: &str) -> String {
    if let Some(inner) = raw
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
    {
        return inner.to_owned();
    }

    match strip_comment_markers(raw) {
        Some(comment) => comment_subject(comment.trim()),
        None => raw.to_owned(),
    }
}

fn strip_comment_markers(raw: &str) -> Option<&str> {
    raw.strip_prefix("//")
        .or_else(|| raw.strip_prefix("/*")?.strip_suffix("*/"))
}

const ESLINT_MARKERS: [&str; 3] = [
    "eslint-disable-next-line",
    "eslint-disable-line",
    "eslint-disable",
];

fn comment_subject(comment: &str) -> String {
    ts_directive(comment)
        .map(str::to_owned)
        .or_else(|| eslint_disable_subject(comment))
        .unwrap_or_else(|| comment.to_owned())
}

fn ts_directive(comment: &str) -> Option<&'static str> {
    ["@ts-ignore", "@ts-nocheck"]
        .into_iter()
        .find(|marker| comment.contains(marker))
        .map(|marker| marker.trim_start_matches('@'))
}

fn eslint_disable_subject(comment: &str) -> Option<String> {
    let (marker, index) = ESLINT_MARKERS
        .into_iter()
        .find_map(|marker| comment.find(marker).map(|index| (marker, index)))?;
    let rest = &comment[index + marker.len()..];
    let rules = rest.split("--").next().unwrap_or(rest).trim();

    Some(if rules.is_empty() {
        marker.to_owned()
    } else {
        rules.to_owned()
    })
}
