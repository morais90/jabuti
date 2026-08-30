use crate::model::{Finding, Rule, Severity, Span};
use crate::policy::Policy;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSummary {
    pub path: String,
    pub span: Span,
    pub churn: u32,
    pub complexity: u32,
}

pub fn hotspots(files: &[FileSummary], policy: &Policy) -> Vec<Finding> {
    let Some(config) = policy.config(Rule::Hotspot) else {
        return Vec::new();
    };
    if config.severity == Severity::Off {
        return Vec::new();
    }

    let churn = ranking(files.iter().map(|file| file.churn));
    let complexity = ranking(files.iter().map(|file| file.complexity));

    let mut findings: Vec<Finding> = files
        .iter()
        .filter_map(|file| {
            let measured = churn.rank(file.churn).min(complexity.rank(file.complexity));
            (measured > config.limit).then(|| Finding {
                rule: Rule::Hotspot,
                severity: config.severity,
                path: file.path.clone(),
                span: file.span,
                subject: None,
                measured,
                limit: config.limit,
            })
        })
        .collect();

    findings.sort_by(|left, right| left.path.cmp(&right.path));
    findings
}

#[derive(Debug)]
struct Ranking {
    sorted: Vec<u32>,
}

fn ranking(values: impl Iterator<Item = u32>) -> Ranking {
    let mut sorted: Vec<u32> = values.collect();
    sorted.sort_unstable();

    Ranking { sorted }
}

impl Ranking {
    fn rank(&self, value: u32) -> u32 {
        if self.sorted.is_empty() {
            return 0;
        }

        let below = self.sorted.partition_point(|other| *other < value);
        let scaled = below * 100 / self.sorted.len();

        u32::try_from(scaled).unwrap_or(100)
    }
}
