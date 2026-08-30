fn straight_line(value: i32) -> i32 {
    value + 1 // cognitive = 0
}

fn single_if(value: i32) -> i32 {
    if value > 0 {
        // +1 if, nesting 0
        1
    } else {
        0
    } // cognitive = 2, the else adds one without a nesting penalty
}

fn else_if_chain(value: i32) -> i32 {
    if value > 10 {
        // +1 if
        2
    } else if value > 0 {
        // +1 else if, no nesting penalty
        1
    } else {
        // +1 else
        0
    } // cognitive = 3
}

fn sequential_ifs(a: bool, b: bool, c: bool) -> i32 {
    let mut total = 0;

    if a {
        // +1 nesting 0
        total += 1;
    }
    if b {
        // +1 nesting 0
        total += 1;
    }
    if c {
        // +1 nesting 0
        total += 1;
    }

    total // cognitive = 3
}

fn nested_ifs(a: bool, b: bool, c: bool) -> i32 {
    if a {
        // +1 nesting 0
        if b {
            // +2 nesting 1
            if c {
                // +3 nesting 2
                return 1;
            }
        }
    }

    0 // cognitive = 6, the same three conditions as above cost twice as much
}

fn nested_flow(items: &[i32], limit: i32) -> i32 {
    let mut total = 0;

    if limit > 0 {
        // +1 nesting 0
        for item in items {
            // +2 nesting 1
            while total < limit {
                // +3 nesting 2
                total += item;
            }
        }
    }

    total // cognitive = 6
}

fn wide_match(value: Option<i32>) -> i32 {
    match value {
        // +1 for the whole match, however many arms it has
        Some(0) => 0,
        Some(1) => 1,
        Some(2) => 2,
        Some(other) => other,
        None => -1,
    } // cognitive = 1
}

fn one_operator_run(a: bool, b: bool, c: bool) -> i32 {
    if a && b && c {
        // +1 if, +1 for the run of &&
        1
    } else {
        // +1 else
        0
    } // cognitive = 3
}

fn mixed_operator_runs(a: bool, b: bool, c: bool) -> i32 {
    if a && b || c {
        // +1 if, +1 for &&, +1 for ||
        1
    } else {
        // +1 else
        0
    } // cognitive = 4
}

fn holds_a_closure(values: &[i32]) -> usize {
    let counted = values.iter().filter(|value| {
        if **value > 0 {
            // +2, the closure raises the nesting level without adding anything itself
            true
        } else {
            // +1 else
            false
        }
    });

    counted.count() // cognitive = 3
}

fn holds_a_nested_function(value: i32) -> i32 {
    fn inner(value: i32) -> i32 {
        if value > 0 {
            // +1, measured on inner and not on its container
            1
        } else {
            // +1 else
            0
        } // inner cognitive = 2
    }

    inner(value) // cognitive = 0
}

fn conditional_inside_an_else_body(a: bool, b: bool) -> i32 {
    if a {
        // +1 if, nesting 0
        0
    } else {
        // +1 else, and its body sits one level deeper
        if b {
            // +2 at nesting 1
            1
        } else {
            // +1 else
            2
        }
    } // cognitive = 5
}
