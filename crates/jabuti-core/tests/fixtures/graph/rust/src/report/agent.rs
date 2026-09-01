use super::theme;
use crate::{config::Settings, git};

pub fn render(settings: &Settings) -> String {
    format!("{} at {}", git::run(&["status"]), settings.root)
}

pub fn width() -> usize {
    theme::colour().len()
}
