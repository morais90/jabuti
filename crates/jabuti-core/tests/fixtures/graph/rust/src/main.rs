use crate::config::Settings;
use crate::report::agent::render;

mod config;
mod git;
mod report;

fn main() {
    let settings = Settings::load();
    let head = git::run(&["rev-parse", "HEAD"]);

    println!("{} {}", render(&settings), head);
}
