pub mod agent;
pub mod theme;

use super::config::Settings;

pub fn summarise(settings: &Settings) -> usize {
    settings.root.len() + agent::render(settings).len() + self::theme::colour().len()
}
