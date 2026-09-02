pub fn run(arguments: &[&str]) -> String {
    arguments.join(" ")
}

pub fn status() -> String {
    run(&["status", "--short"])
}
