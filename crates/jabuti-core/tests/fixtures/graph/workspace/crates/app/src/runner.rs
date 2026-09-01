use engine::model::Settings;

pub fn start() -> String {
    Settings {
        root: engine::version().to_owned(),
    }
    .root
}
