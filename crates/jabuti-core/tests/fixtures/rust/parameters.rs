struct Holder;

fn takes_nothing() {} // parameters = 0

fn takes_two(first: i32, second: i32) -> i32 {
    first + second // parameters = 2
}

impl Holder {
    fn method_ignores_self(&self, first: i32, second: i32) -> i32 {
        first + second // parameters = 2, the receiver is not an argument
    }

    fn method_takes_only_self(&self) -> i32 {
        1 // parameters = 0
    }
}

fn takes_a_closure() -> i32 {
    let add = |first: i32, second: i32| first + second; // parameters = 2

    add(1, 2) // parameters = 0
}
