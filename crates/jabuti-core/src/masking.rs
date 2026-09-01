use crate::lang::LanguageId;
use crate::model::{Detail, Finding, Masking, Rule, RuleId, Severity};
use crate::policy::Policy;

pub fn findings(
    path: &str,
    language: LanguageId,
    maskings: &[Masking],
    policy: &Policy,
) -> Vec<Finding> {
    let Some(config) = policy.config_for(language, Rule::ErrorMasking) else {
        return Vec::new();
    };
    if config.severity == Severity::Off {
        return Vec::new();
    }

    maskings
        .iter()
        .map(|masking| Finding {
            rule: RuleId::Native(Rule::ErrorMasking),
            severity: config.severity,
            path: path.to_owned(),
            span: masking.span,
            subject: Some(masking.construct.clone()),
            detail: Detail::Message {
                message: masking.kind.consequence().to_owned(),
            },
        })
        .collect()
}
