use crate::config::Settings;
use crate::report::render::agent::Line;
use crate::policy::{Policy, Rule as Named};
use super::git;
use super::{scan, tools::probe};
use self::inner::Helper;
use std::collections::BTreeMap;
use serde::Serialize;

mod inner;

pub struct Widget {
    settings: Settings,
    lines: BTreeMap<usize, Line>,
}

impl Widget {
    pub fn draw(&self) -> usize {
        let head = crate::git::run(&["log"]);
        let policy = crate::policy::defaults::strict();
        let helper = super::since::Changes::new();

        self.lines.len() + head + policy + helper
    }

    pub fn describe(&self) -> String {
        format!("{}", crate::tools::probe::name())
    }
}
