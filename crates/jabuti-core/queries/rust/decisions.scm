(if_expression) @decision
(while_expression) @decision
(for_expression) @decision
(loop_expression) @decision
(match_arm) @decision

(binary_expression operator: "&&") @decision
(binary_expression operator: "||") @decision

(match_expression) @decision.discount

(match_pattern condition: (_)) @decision
