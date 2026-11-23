use crate::catalog::{Rule, RuleId};
use crate::lang::LanguageId;
use crate::model::{Detail, Finding, Span};
use crate::policy::Policy;
use crate::tools::coverage::FileCoverage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileUnderCoverage<'a> {
    pub path: &'a str,
    pub language: LanguageId,
    pub compile_time: &'a [Span],
}

pub fn findings(
    file: &FileUnderCoverage<'_>,
    coverage: &FileCoverage,
    added: impl Fn(u32) -> bool,
    policy: &Policy,
) -> Vec<Finding> {
    if policy
        .active_for(file.language, Rule::UncoveredNewCode)
        .is_none()
    {
        return Vec::new();
    }

    let never_run: Vec<Span> = file
        .compile_time
        .iter()
        .copied()
        .filter(|span| {
            coverage
                .lines()
                .all(|(line, hits)| hits == 0 || !within(line, *span))
        })
        .collect();
    let only_compiled = |line: u32| never_run.iter().any(|span| within(line, *span));

    stretches(coverage, |line| added(line) && !only_compiled(line))
        .into_iter()
        .map(|stretch| Finding {
            rule: RuleId::Native(Rule::UncoveredNewCode),
            path: file.path.to_owned(),
            span: stretch.span,
            subject: None,
            detail: Detail::Message {
                message: run_by_no_test(stretch.lines),
            },
        })
        .collect()
}

fn within(line: u32, span: Span) -> bool {
    span.start_line <= line && line <= span.end_line
}

struct Stretch {
    span: Span,
    lines: u32,
}

fn stretches(file: &FileCoverage, added: impl Fn(u32) -> bool) -> Vec<Stretch> {
    let mut found = Vec::new();
    let mut open: Option<Stretch> = None;

    for (line, hits) in file.lines() {
        let uncovered_new_line = hits == 0 && added(line);
        match (&mut open, uncovered_new_line) {
            (Some(stretch), true) => {
                stretch.span.end_line = line;
                stretch.lines += 1;
            }
            (None, true) => {
                open = Some(Stretch {
                    span: Span {
                        start_line: line,
                        end_line: line,
                    },
                    lines: 1,
                });
            }
            (Some(_), false) => found.extend(open.take()),
            (None, false) => {}
        }
    }

    found.extend(open);
    found
}

fn run_by_no_test(lines: u32) -> String {
    if lines == 1 {
        "1 line run by no test".to_owned()
    } else {
        format!("{lines} lines run by no test")
    }
}
