use std::path::Path;

use jabuti_core::catalog::{Rule, RuleId, Severity};
use jabuti_core::lang::LanguageId;
use jabuti_core::model::{Detail, Finding, Span};
use jabuti_core::policy::{Policy, RuleConfig};
use jabuti_core::tools::coverage::{
    self, Coverage, CoverageError, FileCoverage, FileUnderCoverage, Format,
};
use rstest::rstest;

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tools/coverage")
        .join(name);

    std::fs::read_to_string(path).expect("fixture exists")
}

fn lcov() -> Coverage {
    Coverage::parse(Format::Lcov, &fixture("report.lcov")).expect("the fixture is LCOV")
}

fn jacoco() -> Coverage {
    Coverage::parse(Format::Jacoco, &fixture("report.xml")).expect("the fixture is JaCoCo")
}

fn hits_of(file: &FileCoverage, lines: impl IntoIterator<Item = u32>) -> Vec<Option<u32>> {
    lines.into_iter().map(|line| file.hits(line)).collect()
}

#[rstest]
#[case("coverage.lcov", Some(Format::Lcov))]
#[case("lcov.info", Some(Format::Lcov))]
#[case("build/reports/kover/report.xml", Some(Format::Jacoco))]
#[case("coverage.json", None)]
#[case("coverage", None)]
fn the_format_is_read_from_the_extension(#[case] path: &str, #[case] expected: Option<Format>) {
    assert_eq!(Format::of(Path::new(path)), expected);
}

#[test]
fn an_lcov_record_gives_every_instrumented_line_its_hit_count() {
    let file = lcov()
        .file(Path::new("/home/someone/checkout/src/lib.rs"))
        .cloned()
        .expect("the file is in the report");

    assert_eq!(
        hits_of(&file, 1..=14),
        [
            Some(1),
            Some(1),
            Some(1),
            None,
            Some(0),
            Some(0),
            Some(0),
            None,
            Some(0),
            None,
            Some(0),
            None,
            Some(3),
            Some(0),
        ]
    );
}

#[test]
fn a_file_recorded_twice_adds_its_hits_up() {
    let file = lcov()
        .file(Path::new("/home/someone/checkout/src/lib.rs"))
        .cloned()
        .expect("the file is in the report");

    assert_eq!(file.hits(13), Some(3));
}

#[test]
fn an_lcov_line_record_accepts_an_optional_checksum() {
    let coverage = Coverage::parse(
        Format::Lcov,
        "SF:src/lib.rs\nDA:7,2,abc123\nend_of_record\n",
    )
    .expect("valid LCOV");

    assert_eq!(
        coverage
            .file(Path::new("/checkout/src/lib.rs"))
            .and_then(|file| file.hits(7)),
        Some(2)
    );
}

#[rstest]
#[case("", "line number")]
#[case("not-a-line,0", "line number")]
#[case("7", "hit count")]
#[case("7,not-a-hit-count", "hit count")]
fn an_lcov_line_record_rejects_a_missing_or_non_numeric_required_field(
    #[case] record: &str,
    #[case] expected_field: &str,
) {
    let report = format!("SF:src/lib.rs\nDA:{record}\nend_of_record\n");
    let error = Coverage::parse(Format::Lcov, &report).expect_err("invalid LCOV");

    assert!(
        matches!(
            &error,
            CoverageError::InvalidLcovDataRecord {
                record: actual_record,
                field,
            } if actual_record == record && *field == expected_field
        ),
        "{error}"
    );
}

#[test]
fn two_files_sharing_a_name_are_told_apart_by_their_directories() {
    let coverage = lcov();

    assert_eq!(
        coverage
            .file(Path::new("/home/someone/checkout/src/lib.rs"))
            .and_then(|file| file.hits(5)),
        Some(0)
    );
    assert_eq!(
        coverage
            .file(Path::new("/home/someone/checkout/crates/engine/src/lib.rs"))
            .and_then(|file| file.hits(1)),
        Some(0)
    );
    assert_eq!(
        coverage
            .file(Path::new("/home/someone/checkout/src/lib.rs"))
            .and_then(|file| file.hits(1)),
        Some(1)
    );
}

#[test]
fn a_relative_report_path_matches_the_absolute_source_that_ends_with_it() {
    let coverage =
        Coverage::parse(Format::Lcov, "SF:src/lib.rs\nDA:1,0\nend_of_record\n").expect("LCOV");

    assert_eq!(
        coverage
            .file(Path::new("/anywhere/checkout/src/lib.rs"))
            .and_then(|file| file.hits(1)),
        Some(0)
    );
    assert_eq!(
        coverage
            .file(Path::new("/anywhere/checkout/other/lib.rs"))
            .map(|file| file.hits(1)),
        None
    );
}

#[test]
fn a_file_the_report_never_mentions_is_absent_rather_than_uncovered() {
    assert_eq!(
        lcov()
            .file(Path::new("/home/someone/checkout/src/main.rs"))
            .map(|file| file.hits(1)),
        None
    );
}

#[test]
fn text_with_no_source_record_is_not_an_lcov_report() {
    let error = Coverage::parse(Format::Lcov, "TN:\nend_of_record\n").expect_err("not LCOV");

    assert!(matches!(error, CoverageError::NotLcov), "{error}");
}

#[test]
fn a_jacoco_line_reports_its_covered_instruction_count_under_the_package_directory() {
    let file = jacoco()
        .file(Path::new("/checkout/src/main/kotlin/shop/catalog/Shelf.kt"))
        .cloned()
        .expect("the file is in the report");

    assert_eq!(
        hits_of(&file, 3..=12),
        [
            None,
            Some(2),
            Some(1),
            None,
            None,
            Some(0),
            Some(0),
            None,
            None,
            Some(1),
        ]
    );
}

#[test]
fn a_jacoco_file_in_the_default_package_is_found_by_its_name_alone() {
    assert_eq!(
        jacoco()
            .file(Path::new("/checkout/src/main/kotlin/Main.kt"))
            .and_then(|file| file.hits(1)),
        Some(1)
    );
}

fn jacoco_package(contents: &str) -> Result<Coverage, CoverageError> {
    Coverage::parse(
        Format::Jacoco,
        &format!("<report><package name=\"shop\">{contents}</package></report>"),
    )
}

#[test]
fn a_jacoco_source_file_without_a_name_is_rejected() {
    let error = jacoco_package("<sourcefile/>").expect_err("name is required");

    assert!(
        matches!(
            &error,
            CoverageError::MissingJacocoAttribute { element, attribute }
                if *element == "sourcefile" && *attribute == "name"
        ),
        "{error}"
    );
}

#[rstest]
#[case("ci=\"0\"", "nr")]
#[case("nr=\"7\"", "ci")]
fn a_jacoco_line_without_a_required_number_is_rejected(
    #[case] attributes: &str,
    #[case] expected_attribute: &str,
) {
    let source = format!("<sourcefile name=\"Main.kt\"><line {attributes}/></sourcefile>");
    let error = jacoco_package(&source).expect_err("attribute is required");

    assert!(
        matches!(
            &error,
            CoverageError::MissingJacocoAttribute { element, attribute }
                if *element == "line" && *attribute == expected_attribute
        ),
        "{error}"
    );
}

#[rstest]
#[case("nr=\"not-a-line\" ci=\"0\"", "nr", "not-a-line")]
#[case("nr=\"7\" ci=\"not-a-count\"", "ci", "not-a-count")]
fn a_jacoco_line_with_a_non_numeric_required_number_is_rejected(
    #[case] attributes: &str,
    #[case] expected_attribute: &str,
    #[case] expected_value: &str,
) {
    let source = format!("<sourcefile name=\"Main.kt\"><line {attributes}/></sourcefile>");
    let error = jacoco_package(&source).expect_err("attribute must be numeric");

    assert!(
        matches!(
            &error,
            CoverageError::InvalidJacocoNumber {
                element,
                attribute,
                value,
            } if *element == "line"
                && *attribute == expected_attribute
                && value == expected_value
        ),
        "{error}"
    );
}

#[test]
fn xml_that_is_not_a_jacoco_report_is_rejected_by_its_root_element() {
    let error = Coverage::parse(Format::Jacoco, "<coverage/>").expect_err("not JaCoCo");

    assert!(matches!(error, CoverageError::NotJacoco), "{error}");
}

#[test]
fn malformed_xml_is_rejected_with_the_parser_error() {
    let error = Coverage::parse(Format::Jacoco, "<report>").expect_err("not XML");

    assert!(matches!(error, CoverageError::Xml(_)), "{error}");
}

fn reporting(severity: Severity) -> Policy {
    let mut policy = Policy::default();
    policy.set(Rule::UncoveredNewCode, RuleConfig { limit: 0, severity });
    policy
}

fn lib() -> FileCoverage {
    lcov()
        .file(Path::new("/home/someone/checkout/src/lib.rs"))
        .cloned()
        .expect("the file is in the report")
}

fn uncovered(added: impl Fn(u32) -> bool, policy: &Policy) -> Vec<Finding> {
    let file = FileUnderCoverage {
        path: "src/lib.rs",
        language: LanguageId::Rust,
    };

    coverage::findings(&file, &lib(), added, policy)
}

fn finding(start_line: u32, end_line: u32, message: &str) -> Finding {
    Finding {
        rule: RuleId::Native(Rule::UncoveredNewCode),
        severity: Severity::Warning,
        path: "src/lib.rs".to_owned(),
        span: Span {
            start_line,
            end_line,
        },
        subject: None,
        detail: Detail::Message {
            message: message.to_owned(),
        },
    }
}

#[test]
fn consecutive_uncovered_new_lines_are_reported_once_as_a_stretch() {
    let found = uncovered(
        |line| (5..=11).contains(&line),
        &reporting(Severity::Warning),
    );

    assert_eq!(found, [finding(5, 11, "5 lines run by no test")]);
}

#[test]
fn a_covered_line_in_between_splits_the_stretch() {
    let found = uncovered(
        |line| (1..=14).contains(&line),
        &reporting(Severity::Warning),
    );

    assert_eq!(
        found,
        [
            finding(5, 11, "5 lines run by no test"),
            finding(14, 14, "1 line run by no test"),
        ]
    );
}

#[test]
fn a_line_that_was_not_added_is_never_reported_even_when_uncovered() {
    let found = uncovered(|line| line == 14, &reporting(Severity::Warning));

    assert_eq!(found, [finding(14, 14, "1 line run by no test")]);
}

#[test]
fn a_line_no_test_can_reach_is_not_uncovered_when_it_was_not_instrumented() {
    let found = uncovered(|line| line == 4 || line == 8, &reporting(Severity::Warning));

    assert_eq!(found, []);
}

#[test]
fn the_severity_is_the_one_the_policy_gives_the_rule() {
    let severities: Vec<Severity> = uncovered(|_| true, &reporting(Severity::Error))
        .iter()
        .map(|finding| finding.severity)
        .collect();

    assert_eq!(severities, [Severity::Error, Severity::Error]);
}

#[test]
fn a_switched_off_rule_reports_nothing() {
    assert_eq!(uncovered(|_| true, &reporting(Severity::Off)), []);
}

#[test]
fn a_language_that_switches_the_rule_off_reports_nothing_for_its_files() {
    let mut policy = reporting(Severity::Warning);
    policy.set_for(
        LanguageId::Rust,
        Rule::UncoveredNewCode,
        RuleConfig {
            limit: 0,
            severity: Severity::Off,
        },
    );

    assert_eq!(uncovered(|_| true, &policy), []);
}
