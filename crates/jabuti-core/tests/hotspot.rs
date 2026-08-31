use jabuti_core::hotspot::{self, FileSummary};
use jabuti_core::model::{Detail, Finding, Rule, Severity, Span};
use jabuti_core::policy::{Policy, RuleConfig};

fn measured(finding: &Finding) -> u32 {
    match finding.detail {
        Detail::Threshold { measured, .. } => measured,
        Detail::Message(_) => unreachable!("hotspot reports a threshold"),
    }
}

fn file(path: &str, churn: u32, complexity: u32) -> FileSummary {
    FileSummary {
        path: path.to_owned(),
        span: Span {
            start_line: 1,
            end_line: 100,
        },
        churn,
        complexity,
    }
}

fn reporting_above(limit: u32) -> Policy {
    let mut policy = Policy::default();
    policy.set(
        Rule::Hotspot,
        RuleConfig {
            limit,
            severity: Severity::Warning,
        },
    );
    policy
}

fn spread() -> Vec<FileSummary> {
    (0..10)
        .map(|index| file(&format!("src/f{index}.rs"), index, index))
        .collect()
}

fn reported(files: &[FileSummary], limit: u32) -> Vec<(String, u32)> {
    hotspot::hotspots(files, &reporting_above(limit))
        .into_iter()
        .map(|finding| (finding.path.clone(), measured(&finding)))
        .collect()
}

#[test]
fn a_file_high_on_both_axes_is_reported() {
    let mut files = spread();
    files.push(file("src/busy_and_complex.rs", 100, 100));

    assert_eq!(
        reported(&files, 85),
        [("src/busy_and_complex.rs".to_owned(), 90)]
    );
}

#[test]
fn changing_often_without_being_complex_is_not_a_hotspot() {
    let mut files = spread();
    files.push(file("src/busy_but_simple.rs", 100, 0));

    assert_eq!(reported(&files, 85), []);
}

#[test]
fn being_complex_without_changing_is_not_a_hotspot() {
    let mut files = spread();
    files.push(file("src/complex_but_still.rs", 0, 100));

    assert_eq!(reported(&files, 85), []);
}

#[test]
fn the_reported_number_is_the_lower_of_the_two_rankings() {
    let mut files = spread();
    files.push(file("src/lopsided.rs", 100, 5));

    let ranks = hotspot::hotspots(&files, &reporting_above(0));
    let lopsided = ranks
        .iter()
        .find(|finding| finding.path == "src/lopsided.rs")
        .expect("lopsided is ranked");

    assert_eq!(measured(lopsided), 45);
}

#[test]
fn a_rule_switched_off_ranks_nothing() {
    let mut policy = Policy::default();
    policy.set(
        Rule::Hotspot,
        RuleConfig {
            limit: 0,
            severity: Severity::Off,
        },
    );

    assert_eq!(hotspot::hotspots(&spread(), &policy), []);
}

#[test]
fn a_single_file_cannot_stand_out_from_anything() {
    assert_eq!(reported(&[file("src/only.rs", 999, 999)], 0), []);
}
