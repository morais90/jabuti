use tree_sitter::Node;

use super::lang::{CognitiveSpec, ConditionalSpec};
use super::metrics::Increment;

pub(crate) fn increments(root: Node<'_>, spec: &CognitiveSpec) -> Vec<Increment> {
    let mut walk = Walk {
        spec,
        found: Vec::new(),
    };

    walk.run(root);
    walk.found.sort_by_key(|increment| increment.position);
    walk.found
}

struct Walk<'spec> {
    spec: &'spec CognitiveSpec,
    found: Vec<Increment>,
}

#[derive(Clone, Copy)]
struct Pending<'tree> {
    node: Node<'tree>,
    nesting: u32,
    conditional_amount: Option<u32>,
}

impl Walk<'_> {
    fn run(&mut self, root: Node<'_>) {
        let mut pending = vec![Pending {
            node: root,
            nesting: 0,
            conditional_amount: None,
        }];

        while let Some(next) = pending.pop() {
            self.visit(next, &mut pending);
        }
    }

    fn visit<'tree>(&mut self, next: Pending<'tree>, pending: &mut Vec<Pending<'tree>>) {
        let node = next.node;
        let kind = node.kind();

        if self.is_boundary(node) {
            push_children(pending, node, 0, None);
            return;
        }

        if let Some(conditional) = self
            .spec
            .conditionals
            .iter()
            .find(|conditional| conditional.kind == kind)
        {
            self.visit_conditional(next, conditional, pending);
            return;
        }

        if self.spec.nesting_increments.contains(&kind) {
            self.record(node, 1 + next.nesting);
            push_children(pending, node, next.nesting + 1, None);
            return;
        }

        if self.spec.nesting_only.contains(&kind) {
            push_children(pending, node, next.nesting + 1, None);
            return;
        }

        if self.starts_a_logical_sequence(node) {
            self.record(node, 1);
        }

        push_children(pending, node, next.nesting, None);
    }

    fn is_boundary(&self, node: Node<'_>) -> bool {
        if self.spec.boundaries.contains(&node.kind()) {
            return true;
        }

        let Some(parent) = node.parent() else {
            return false;
        };

        self.spec.contextual_boundaries.iter().any(|boundary| {
            boundary.kind == node.kind() && boundary.parents.contains(&parent.kind())
        })
    }

    fn visit_conditional<'tree>(
        &mut self,
        next: Pending<'tree>,
        conditional: &ConditionalSpec,
        pending: &mut Vec<Pending<'tree>>,
    ) {
        let node = next.node;
        let nesting = next.nesting;
        let amount = next.conditional_amount.unwrap_or(1 + nesting);
        self.record(node, amount);

        let alternative = self.alternative(node);
        push_children(
            pending,
            node,
            nesting + 1,
            alternative.map(|branch| branch.id()),
        );

        if let Some(branch) = Self::otherwise(alternative, conditional) {
            self.schedule_alternative(branch, nesting, conditional, pending);
        }
    }

    fn schedule_alternative<'tree>(
        &mut self,
        branch: Node<'tree>,
        nesting: u32,
        conditional: &ConditionalSpec,
        pending: &mut Vec<Pending<'tree>>,
    ) {
        if conditional.chains_alternative && branch.kind() == conditional.kind {
            pending.push(Pending {
                node: branch,
                nesting,
                conditional_amount: Some(1),
            });
            return;
        }

        if conditional.charge_alternative {
            self.record(branch, 1);
        }
        pending.push(Pending {
            node: branch,
            nesting: nesting + 1,
            conditional_amount: None,
        });
    }

    fn alternative<'tree>(&self, node: Node<'tree>) -> Option<Node<'tree>> {
        let condition = node.child_by_field_name(self.spec.condition_field);
        let mut cursor = node.walk();

        node.named_children(&mut cursor)
            .filter(|child| Some(child.id()) != condition.map(|node| node.id()))
            .nth(1)
    }

    fn otherwise<'tree>(
        alternative: Option<Node<'tree>>,
        conditional: &ConditionalSpec,
    ) -> Option<Node<'tree>> {
        let alternative = alternative?;

        if alternative.kind() == conditional.alternative_wrapper {
            alternative.named_child(0)
        } else {
            Some(alternative)
        }
    }

    fn starts_a_logical_sequence(&self, node: Node<'_>) -> bool {
        let Some(operator) = self.logical_operator(node) else {
            return false;
        };

        match node.parent() {
            Some(parent) => self.logical_operator(parent) != Some(operator),
            None => true,
        }
    }

    fn logical_operator<'tree>(&self, node: Node<'tree>) -> Option<&'tree str> {
        if node.kind() != self.spec.logical_expression {
            return None;
        }

        let operator = node.child_by_field_name(self.spec.operator_field)?.kind();
        self.spec
            .logical_operators
            .contains(&operator)
            .then_some(operator)
    }

    fn record(&mut self, node: Node<'_>, amount: u32) {
        self.found.push(Increment {
            position: node.start_byte(),
            amount,
        });
    }
}

fn push_children<'tree>(
    pending: &mut Vec<Pending<'tree>>,
    node: Node<'tree>,
    nesting: u32,
    skipped: Option<usize>,
) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if Some(child.id()) != skipped {
            pending.push(Pending {
                node: child,
                nesting,
                conditional_amount: None,
            });
        }
    }
}
