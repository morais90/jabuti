use super::concepts::Occurrence;
use crate::lang::LanguageId;
use crate::model::{Concept, Detail, Finding, Rule, RuleId, Severity};
use crate::policy::Policy;

pub fn findings(
    path: &str,
    language: LanguageId,
    occurrences: &[Occurrence],
    policy: &Policy,
) -> Vec<Finding> {
    let Some(config) = policy.config_for(language, Rule::ErrorMasking) else {
        return Vec::new();
    };
    if config.severity == Severity::Off {
        return Vec::new();
    }

    occurrences
        .iter()
        .map(|occurrence| Finding {
            rule: RuleId::Native(Rule::ErrorMasking),
            severity: config.severity,
            path: path.to_owned(),
            span: occurrence.span,
            subject: Some(occurrence.subject.clone()),
            detail: Detail::Message {
                message: consequence(occurrence.concept).to_owned(),
            },
        })
        .collect()
}

fn consequence(concept: Concept) -> &'static str {
    match concept {
        Concept::ErrorDiscard => "the failure is dropped without being read",
        Concept::ErrorPanic => "the failure becomes a panic",
        Concept::ErrorSwallow => "the failure is caught and nothing happens",
    }
}
