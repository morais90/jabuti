use engine::model::Settings;

mod runner;

fn main() {
    let settings = Settings {
        root: engine::version().to_owned(),
    };

    println!("{} {}", settings.root, runner::start());
}
