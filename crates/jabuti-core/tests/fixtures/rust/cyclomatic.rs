fn straight_line(value: i32) -> i32 {
    value + 1 // cyclomatic = 1
}

fn single_branch(value: i32) -> i32 {
    if value > 0 {
        // +1 if
        1
    } else {
        0
    } // cyclomatic = 2
}

fn else_if_chain(value: i32) -> i32 {
    if value > 10 {
        // +1 if
        2
    } else if value > 0 {
        // +1 if
        1
    } else {
        0
    } // cyclomatic = 3
}

fn boolean_operators(a: bool, b: bool, c: bool) -> bool {
    if a && b || c {
        // +1 if, +1 &&, +1 ||
        true
    } else {
        false
    } // cyclomatic = 4
}

fn every_loop(limit: usize) -> usize {
    let mut total = 0;

    for _ in 0..limit {
        // +1 for
        total += 1;
    }

    while total > limit {
        // +1 while
        total -= 1;
    }

    loop {
        // +1 loop
        break;
    }

    total // cyclomatic = 4
}

fn one_arm_match(value: Option<i32>) -> i32 {
    match value {
        // -1 discount
        _ => 0, // +1 arm
    } // cyclomatic = 1
}

fn three_arm_match(value: Option<i32>) -> i32 {
    match value {
        // -1 discount
        Some(0) => 0,         // +1 arm
        Some(other) => other, // +1 arm
        None => -1,           // +1 arm
    } // cyclomatic = 3
}

fn holds_a_closure(values: &[i32]) -> usize {
    let positive = |value: &i32| {
        if *value > 0 {
            // +1 if, inside a closure, counted here
            true
        } else {
            false
        }
    };

    values.iter().filter(|value| positive(value)).count() // cyclomatic = 2
}

fn holds_a_nested_function(value: i32) -> i32 {
    fn inner(value: i32) -> i32 {
        if value > 0 {
            // +1 if, measured on inner, not on its container
            1
        } else {
            0
        } // inner cyclomatic = 2
    }

    inner(value) // cyclomatic = 1
}

fn guarded_match(value: Option<i32>) -> i32 {
    match value {
        // -1 discount
        Some(x) if x > 0 => x, // +1 arm, +1 guard
        Some(_) => 0,          // +1 arm
        None => -1,            // +1 arm
    } // cyclomatic = 4
}
