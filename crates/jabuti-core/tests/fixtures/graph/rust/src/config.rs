use std::collections::BTreeMap;

pub struct Settings {
    pub root: String,
    pub overrides: BTreeMap<String, String>,
}

impl Settings {
    pub fn load() -> Self {
        Self {
            root: crate::git::run(&["rev-parse", "--show-toplevel"]),
            overrides: BTreeMap::new(),
        }
    }
}
