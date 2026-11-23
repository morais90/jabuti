use jabuti_core::catalog::{Input, Rule};

use crate::config::Settings;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Plan<'a> {
    settings: &'a Settings,
    compared: bool,
}

impl<'a> Plan<'a> {
    pub(crate) fn of(settings: &'a Settings, compared: bool) -> Self {
        Self { settings, compared }
    }

    pub(crate) fn runs(self, rule: Rule) -> bool {
        let spec = rule.spec();

        self.settings.policy.enabled(rule)
            && spec.scoping.allows(self.compared)
            && spec.inputs.iter().all(|input| self.available(*input))
    }

    pub(crate) fn reads(self, input: Input) -> bool {
        Rule::ALL
            .into_iter()
            .any(|rule| self.runs(rule) && rule.spec().inputs.contains(&input))
    }

    fn available(self, input: Input) -> bool {
        match input {
            Input::Layers => !self.settings.layers.is_empty(),
            Input::BaseRevision | Input::Graph | Input::History => true,
        }
    }
}
