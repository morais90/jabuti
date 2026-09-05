use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use roxmltree::Node;
use thiserror::Error;

use crate::lang::LanguageId;
use crate::model::{Detail, Finding, Rule, RuleId, Severity, Span};
use crate::policy::Policy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Lcov,
    Jacoco,
}

impl Format {
    pub fn of(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "lcov" | "info" => Some(Self::Lcov),
            "xml" => Some(Self::Jacoco),
            _ => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum CoverageError {
    #[error("not well-formed XML: {0}")]
    Xml(#[from] roxmltree::Error),
    #[error("LCOV DA record {record:?} has an invalid {field}")]
    InvalidLcovDataRecord { record: String, field: &'static str },
    #[error("JaCoCo <{element}> is missing its {attribute} attribute")]
    MissingJacocoAttribute {
        element: &'static str,
        attribute: &'static str,
    },
    #[error("JaCoCo <{element}> has a non-numeric {attribute} attribute {value:?}")]
    InvalidJacocoNumber {
        element: &'static str,
        attribute: &'static str,
        value: String,
    },
    #[error("no <report> element, so this is not a JaCoCo report")]
    NotJacoco,
    #[error("no SF: record, so this is not an LCOV report")]
    NotLcov,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FileCoverage {
    hits: BTreeMap<u32, u32>,
}

impl FileCoverage {
    pub fn hits(&self, line: u32) -> Option<u32> {
        self.hits.get(&line).copied()
    }

    fn record(&mut self, line: u32, hits: u32) {
        *self.hits.entry(line).or_default() += hits;
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Coverage {
    files: BTreeMap<PathBuf, FileCoverage>,
}

impl Coverage {
    pub fn parse(format: Format, text: &str) -> Result<Self, CoverageError> {
        match format {
            Format::Lcov => lcov(text),
            Format::Jacoco => jacoco(text),
        }
    }

    pub fn file(&self, path: &Path) -> Option<&FileCoverage> {
        self.files
            .iter()
            .filter(|(recorded, _)| path.ends_with(recorded))
            .max_by_key(|(recorded, _)| recorded.components().count())
            .map(|(_, file)| file)
    }

    fn entry(&mut self, path: PathBuf) -> &mut FileCoverage {
        self.files.entry(path).or_default()
    }
}

fn lcov(text: &str) -> Result<Coverage, CoverageError> {
    let mut coverage = Coverage::default();
    let mut current: Option<PathBuf> = None;

    for line in text.lines() {
        if let Some(source) = line.strip_prefix("SF:") {
            current = Some(PathBuf::from(source));
            coverage.entry(PathBuf::from(source));
        } else if let Some(data) = line.strip_prefix("DA:")
            && let Some(path) = &current
        {
            let (number, hits) = line_and_hits(data)?;
            coverage.entry(path.clone()).record(number, hits);
        } else if line == "end_of_record" {
            current = None;
        }
    }

    if coverage.files.is_empty() {
        return Err(CoverageError::NotLcov);
    }

    Ok(coverage)
}

fn line_and_hits(data: &str) -> Result<(u32, u32), CoverageError> {
    let mut fields = data.split(',');
    let line = fields
        .next()
        .ok_or_else(|| invalid_lcov_data(data, "line number"))?
        .trim()
        .parse()
        .map_err(|_| invalid_lcov_data(data, "line number"))?;
    let hits = fields
        .next()
        .ok_or_else(|| invalid_lcov_data(data, "hit count"))?
        .trim()
        .parse()
        .map_err(|_| invalid_lcov_data(data, "hit count"))?;

    Ok((line, hits))
}

fn invalid_lcov_data(record: &str, field: &'static str) -> CoverageError {
    CoverageError::InvalidLcovDataRecord {
        record: record.to_owned(),
        field,
    }
}

fn jacoco(text: &str) -> Result<Coverage, CoverageError> {
    let document = roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        },
    )?;
    let report = document.root_element();
    if report.tag_name().name() != "report" {
        return Err(CoverageError::NotJacoco);
    }

    let mut coverage = Coverage::default();
    for package in children_named(report, "package") {
        record_package(&mut coverage, package)?;
    }

    Ok(coverage)
}

fn record_package(coverage: &mut Coverage, package: Node<'_, '_>) -> Result<(), CoverageError> {
    let directory = package.attribute("name").unwrap_or_default();

    for source in children_named(package, "sourcefile") {
        let name = source
            .attribute("name")
            .ok_or(CoverageError::MissingJacocoAttribute {
                element: "sourcefile",
                attribute: "name",
            })?;
        record_source(coverage.entry(Path::new(directory).join(name)), source)?;
    }

    Ok(())
}

fn record_source(file: &mut FileCoverage, source: Node<'_, '_>) -> Result<(), CoverageError> {
    for line in children_named(source, "line") {
        let number = numeric(line, "nr")?;
        let covered = numeric(line, "ci")?;
        file.record(number, covered);
    }

    Ok(())
}

fn children_named<'a, 'input>(
    node: Node<'a, 'input>,
    name: &'a str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children()
        .filter(move |child| child.tag_name().name() == name)
}

fn numeric(node: Node<'_, '_>, attribute: &'static str) -> Result<u32, CoverageError> {
    let value = node
        .attribute(attribute)
        .ok_or(CoverageError::MissingJacocoAttribute {
            element: "line",
            attribute,
        })?;

    value
        .parse()
        .map_err(|_| CoverageError::InvalidJacocoNumber {
            element: "line",
            attribute,
            value: value.to_owned(),
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileUnderCoverage<'a> {
    pub path: &'a str,
    pub language: LanguageId,
}

pub fn findings(
    file: &FileUnderCoverage<'_>,
    coverage: &FileCoverage,
    added: impl Fn(u32) -> bool,
    policy: &Policy,
) -> Vec<Finding> {
    let Some(config) = policy.config_for(file.language, Rule::UncoveredNewCode) else {
        return Vec::new();
    };
    if config.severity == Severity::Off {
        return Vec::new();
    }

    stretches(coverage, added)
        .into_iter()
        .map(|stretch| Finding {
            rule: RuleId::Native(Rule::UncoveredNewCode),
            severity: config.severity,
            path: file.path.to_owned(),
            span: stretch.span,
            subject: None,
            detail: Detail::Message {
                message: run_by_no_test(stretch.lines),
            },
        })
        .collect()
}

struct Stretch {
    span: Span,
    lines: u32,
}

fn stretches(file: &FileCoverage, added: impl Fn(u32) -> bool) -> Vec<Stretch> {
    let mut found = Vec::new();
    let mut open: Option<Stretch> = None;

    for (&line, &hits) in &file.hits {
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
