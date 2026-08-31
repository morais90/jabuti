use tree_sitter::Node;

use crate::lang::CognitiveSpec;
use crate::model::Increment;

pub(crate) fn increments(root: Node<'_>, spec: &CognitiveSpec) -> Vec<Increment> {
    let mut walk = Walk {
        spec,
        found: Vec::new(),
    };

    walk.visit(root, 0);
    walk.found.sort_by_key(|increment| increment.position);
    walk.found
}

struct Walk<'spec> {
    spec: &'spec CognitiveSpec,
    found: Vec<Increment>,
}

impl Walk<'_> {
    fn visit(&mut self, node: Node<'_>, nesting: u32) {
        let kind = node.kind();

        if self.spec.boundaries.contains(&kind) {
            self.visit_children(node, 0);
            return;
        }

        if kind == self.spec.conditional {
            self.visit_conditional(node, nesting, 1 + nesting);
            return;
        }

        if self.spec.nesting_increments.contains(&kind) {
            self.record(node, 1 + nesting);
            self.visit_children(node, nesting + 1);
            return;
        }

        if self.spec.nesting_only.contains(&kind) {
            self.visit_children(node, nesting + 1);
            return;
        }

        if self.starts_a_logical_sequence(node) {
            self.record(node, 1);
        }

        self.visit_children(node, nesting);
    }

    fn visit_conditional(&mut self, node: Node<'_>, nesting: u32, amount: u32) {
        self.record(node, amount);

        let alternative = self.alternative(node);
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if Some(child.id()) != alternative.map(|node| node.id()) {
                self.visit(child, nesting + 1);
            }
        }

        if let Some(branch) = self.otherwise(alternative) {
            if branch.kind() == self.spec.conditional {
                self.visit_conditional(branch, nesting, 1);
            } else {
                self.record(branch, 1);
                self.visit(branch, nesting + 1);
            }
        }
    }

    fn alternative<'tree>(&self, node: Node<'tree>) -> Option<Node<'tree>> {
        let condition = node.child_by_field_name(self.spec.condition_field);
        let mut cursor = node.walk();

        node.named_children(&mut cursor)
            .filter(|child| Some(child.id()) != condition.map(|node| node.id()))
            .nth(1)
    }

    fn otherwise<'tree>(&self, alternative: Option<Node<'tree>>) -> Option<Node<'tree>> {
        let alternative = alternative?;

        if alternative.kind() == self.spec.alternative_wrapper {
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

    fn visit_children(&mut self, node: Node<'_>, nesting: u32) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.visit(child, nesting);
        }
    }

    fn record(&mut self, node: Node<'_>, amount: u32) {
        self.found.push(Increment {
            position: node.start_byte(),
            amount,
        });
    }
}
