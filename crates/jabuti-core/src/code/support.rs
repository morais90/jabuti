use super::lang;
use crate::catalog::{Portability, Rule};
use crate::lang::LanguageId;

pub fn portability_available(
    portability: Portability,
    has_concept: bool,
    language_specific: bool,
) -> bool {
    match portability {
        Portability::Universal => true,
        Portability::ConceptBound => has_concept,
        Portability::LanguageSpecific => language_specific,
    }
}

pub fn available_rules(language: LanguageId) -> Vec<Rule> {
    let table = lang::table(language);

    Rule::ALL
        .into_iter()
        .filter(|rule| {
            let has_concept = rule
                .spec()
                .concepts
                .iter()
                .any(|concept| table.concepts.contains(concept));
            let language_specific = table.language_specific_rules.contains(rule);

            portability_available(rule.portability(), has_concept, language_specific)
        })
        .collect()
}
