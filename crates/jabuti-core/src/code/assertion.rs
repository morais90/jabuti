use super::concepts::{self, CallSite, Occurrence};
use super::units::Unit;
use crate::catalog::{Rule, RuleId, Severity, UnitKind};
use crate::model::{Detail, Finding, Span};
use crate::policy::Policy;
use crate::syntax::Parsed;

#[derive(Debug)]
pub struct Input<'a> {
    pub path: &'a str,
    pub parsed: &'a Parsed<'a>,
    pub unit: &'a Unit,
    pub occurrences: &'a [Occurrence],
}

pub fn findings(input: &Input<'_>, policy: &Policy) -> Vec<Finding> {
    let Some(config) = policy.active_for(input.parsed.language(), Rule::Assertion) else {
        return Vec::new();
    };

    let concepts = Rule::Assertion.spec().concepts;
    let assertions: Vec<&Occurrence> = input
        .occurrences
        .iter()
        .filter(|occurrence| concepts.contains(&occurrence.concept))
        .collect();

    let lookup = Lookup {
        path: input.path,
        severity: config.severity,
        assertions,
        calls: concepts::call_sites(input.parsed),
    };

    let mut findings = Vec::new();
    lookup.collect(input.unit, &[], &mut findings);
    findings
}

struct Lookup<'a> {
    path: &'a str,
    severity: Severity,
    assertions: Vec<&'a Occurrence>,
    calls: Vec<CallSite>,
}

impl Lookup<'_> {
    fn collect<'a>(
        &self,
        unit: &'a Unit,
        ancestor_scopes: &[&'a [Unit]],
        findings: &mut Vec<Finding>,
    ) {
        if unit.is_test && !self.asserts(unit, ancestor_scopes) {
            findings.push(Finding {
                rule: RuleId::Native(Rule::Assertion),
                severity: self.severity,
                path: self.path.to_owned(),
                span: unit.span,
                subject: unit.name.clone(),
                detail: Detail::Message {
                    message: "the test runs and asserts nothing".to_owned(),
                },
            });
        }

        let child_scopes: Vec<&[Unit]> = ancestor_scopes
            .iter()
            .copied()
            .chain([unit.children.as_slice()])
            .collect();
        for child in &unit.children {
            self.collect(child, &child_scopes, findings);
        }
    }

    fn asserts(&self, unit: &Unit, ancestor_scopes: &[&[Unit]]) -> bool {
        if unit.should_panic || self.directly_asserts(unit.span) {
            return true;
        }

        self.calls
            .iter()
            .filter(|call| within(call.span, unit.span))
            .any(|call| {
                ancestor_scopes
                    .iter()
                    .flat_map(|scope| scope.iter())
                    .find(|candidate| {
                        candidate.kind == UnitKind::Function
                            && candidate.name.as_deref() == Some(call.name.as_str())
                    })
                    .is_some_and(|helper| self.directly_asserts(helper.span))
            })
    }

    fn directly_asserts(&self, span: Span) -> bool {
        self.assertions
            .iter()
            .any(|occurrence| within(occurrence.span, span))
    }
}

fn within(inner: Span, outer: Span) -> bool {
    outer.start_line <= inner.start_line && inner.end_line <= outer.end_line
}
