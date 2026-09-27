use jabuti_core::catalog::{Input, Rule, Scoping};

use crate::config::Settings;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Plan<'a> {
    settings: &'a Settings,
    scoped: bool,
}

impl<'a> Plan<'a> {
    pub(crate) fn of(settings: &'a Settings, scoped: bool) -> Self {
        Self { settings, scoped }
    }

    pub(crate) fn runs(self, rule: Rule) -> bool {
        let spec = rule.spec();

        self.settings.policy.enabled(rule)
            && spec.scoping.allows(self.scoped)
            && spec.inputs.iter().all(|input| self.available(*input))
    }

    pub(crate) fn reads(self, input: Input) -> bool {
        Rule::ALL
            .into_iter()
            .any(|rule| self.runs(rule) && rule.spec().inputs.contains(&input))
    }

    pub(crate) fn notices(self) -> Vec<String> {
        let policy = &self.settings.policy;
        let mut notices = Vec::new();

        for rule in Rule::ALL {
            match rule.spec().scoping {
                Scoping::Repository if self.scoped && policy.enabled(rule) => {
                    notices.push(format!(
                        "{} ranks a whole repository, so it is not evaluated with --since",
                        rule.id()
                    ));
                }
                Scoping::Change if !self.scoped && policy.gates(rule) => notices.push(format!(
                    "{} compares against an earlier revision, so it needs --since",
                    rule.id()
                )),
                Scoping::Any | Scoping::Change | Scoping::Repository => {}
            }
        }

        notices
    }

    fn available(self, input: Input) -> bool {
        match input {
            Input::Layers => !self.settings.layers.is_empty(),
            Input::BaseRevision | Input::Graph | Input::History => true,
        }
    }
}
