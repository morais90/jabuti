use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::Path;

use serde::Serialize;

use crate::model::{Detail, Finding, RuleId, Severity, Unreadable};

const INFORMATION_URI: &str = "https://github.com/morais90/jabuti";
const SCHEMA: &str =
    "https://docs.oasis-open.org/sarif/sarif/v2.1.0/errata01/os/schemas/sarif-schema-2.1.0.json";
const VERSION: &str = "2.1.0";

#[derive(Debug, Serialize)]
struct SarifLog<'a> {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: [Run<'a>; 1],
}

#[derive(Debug, Serialize)]
struct Run<'a> {
    tool: Tool<'a>,
    invocations: [Invocation<'a>; 1],
    results: Vec<SarifResult<'a>>,
}

#[derive(Debug, Serialize)]
struct Tool<'a> {
    driver: Driver<'a>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Driver<'a> {
    name: &'static str,
    semantic_version: &'static str,
    information_uri: &'static str,
    rules: Vec<RuleDescriptor<'a>>,
}

#[derive(Debug, Serialize)]
struct RuleDescriptor<'a> {
    id: &'a str,
}

#[derive(Debug)]
struct IdentifiedRule<'a> {
    rule: &'a RuleId,
    id: String,
    descriptor_index: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Invocation<'a> {
    execution_successful: bool,
    tool_execution_notifications: Vec<Notification<'a>>,
}

#[derive(Debug, Serialize)]
struct Notification<'a> {
    level: &'static str,
    message: Message<'a>,
    locations: [Location; 1],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult<'a> {
    rule_id: &'a str,
    rule_index: usize,
    level: &'static str,
    message: Message<'a>,
    locations: [Location; 1],
}

#[derive(Debug, Serialize)]
struct Message<'a> {
    text: Cow<'a, str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Location {
    physical_location: PhysicalLocation,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PhysicalLocation {
    artifact_location: ArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<Region>,
}

#[derive(Debug, Serialize)]
struct ArtifactLocation {
    uri: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct Region {
    start_line: u32,
    end_line: u32,
}

pub(super) fn render(findings: &[Finding], unreadable: &[Unreadable]) -> String {
    let rules = identified_rules(findings);
    let rule_identifiers: BTreeMap<&RuleId, (&str, usize)> = rules
        .iter()
        .map(|rule| (rule.rule, (rule.id.as_str(), rule.descriptor_index)))
        .collect();
    let mut descriptors = Vec::new();
    for rule in &rules {
        if descriptors.len() == rule.descriptor_index {
            descriptors.push(RuleDescriptor { id: &rule.id });
        }
    }
    let results = findings
        .iter()
        .map(|finding| {
            let &(rule_id, rule_index) = rule_identifiers
                .get(&finding.rule)
                .expect("every finding rule is indexed");

            SarifResult {
                rule_id,
                rule_index,
                level: level(finding.severity),
                message: Message {
                    text: result_message(finding),
                },
                locations: [location(
                    &finding.path,
                    Some(Region {
                        start_line: finding.span.start_line,
                        end_line: finding.span.end_line,
                    }),
                )],
            }
        })
        .collect();
    let log = SarifLog {
        schema: SCHEMA,
        version: VERSION,
        runs: [Run {
            tool: Tool {
                driver: Driver {
                    name: "jabuti",
                    semantic_version: env!("CARGO_PKG_VERSION"),
                    information_uri: INFORMATION_URI,
                    rules: descriptors,
                },
            },
            invocations: [Invocation {
                execution_successful: true,
                tool_execution_notifications: notifications(unreadable),
            }],
            results,
        }],
    };

    super::rendered(&log)
}

fn identified_rules(findings: &[Finding]) -> Vec<IdentifiedRule<'_>> {
    let unique: BTreeSet<&RuleId> = findings.iter().map(|finding| &finding.rule).collect();
    let mut identified: Vec<IdentifiedRule<'_>> = unique
        .into_iter()
        .map(|rule| IdentifiedRule {
            rule,
            id: rule.id(),
            descriptor_index: 0,
        })
        .collect();
    identified.sort_unstable_by(|left, right| left.id.cmp(&right.id));

    let mut descriptor_count = 0;
    for position in 0..identified.len() {
        if position == 0 || identified[position - 1].id != identified[position].id {
            descriptor_count += 1;
        }
        identified[position].descriptor_index = descriptor_count - 1;
    }

    identified
}

fn notifications(unreadable: &[Unreadable]) -> Vec<Notification<'_>> {
    let mut files: Vec<&Unreadable> = unreadable.iter().collect();
    files.sort_unstable_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.reason.cmp(&right.reason))
    });

    files
        .into_iter()
        .map(|file| Notification {
            level: "warning",
            message: Message {
                text: Cow::Borrowed(&file.reason),
            },
            locations: [location(&file.path, None)],
        })
        .collect()
}

fn result_message(finding: &Finding) -> Cow<'_, str> {
    if finding.subject.is_none()
        && let Detail::Message { message } = &finding.detail
    {
        return Cow::Borrowed(message);
    }

    let mut message = String::new();
    if let Some(subject) = &finding.subject {
        message.push_str(subject);
        message.push(' ');
    }
    super::write_detail(&mut message, &finding.detail);

    Cow::Owned(message)
}

fn level(severity: Severity) -> &'static str {
    match severity {
        Severity::Warning => "warning",
        Severity::Error => "error",
        Severity::Off => unreachable!("off findings are not reported"),
    }
}

fn location(path: &str, region: Option<Region>) -> Location {
    Location {
        physical_location: PhysicalLocation {
            artifact_location: ArtifactLocation {
                uri: artifact_uri(path),
            },
            region,
        },
    }
}

fn artifact_uri(path: &str) -> String {
    let mut uri = String::with_capacity(path.len());
    for component in Path::new(path).components() {
        if !uri.is_empty() {
            uri.push('/');
        }
        let component = component
            .as_os_str()
            .to_str()
            .expect("a UTF-8 path has UTF-8 components");
        encode_component(&mut uri, component);
    }

    uri
}

fn encode_component(uri: &mut String, component: &str) {
    for byte in component.bytes() {
        if uri_byte(byte) {
            uri.push(char::from(byte));
        } else {
            write!(uri, "%{byte:02X}").expect("writing to a string never fails");
        }
    }
}

fn uri_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}
