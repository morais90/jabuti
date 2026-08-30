use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use jabuti_core::model::{Rule, Severity};
use jabuti_core::policy::{Policy, RuleConfig};
use serde::Deserialize;

pub(crate) const FILE_NAME: &str = "jabuti.toml";

#[derive(Debug, Default)]
pub(crate) struct Settings {
    pub(crate) policy: Policy,
    pub(crate) exclude: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Document {
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    rules: BTreeMap<String, Entry>,
}

#[derive(Debug, Deserialize)]
struct Entry {
    limit: Option<u32>,
    severity: Option<String>,
}

pub(crate) fn load(directory: &Path) -> Result<Settings> {
    let path = directory.join(FILE_NAME);
    if !path.exists() {
        return Ok(Settings::default());
    }

    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let document: Document =
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;

    settings(document)
}

fn settings(document: Document) -> Result<Settings> {
    let mut policy = Policy::default();

    for (id, entry) in document.rules {
        let rule = Rule::from_id(&id).with_context(|| format!("unknown rule {id}"))?;
        let current = policy.config(rule).unwrap_or(RuleConfig {
            limit: 0,
            severity: Severity::Warning,
        });

        policy.set(
            rule,
            RuleConfig {
                limit: entry.limit.unwrap_or(current.limit),
                severity: match entry.severity {
                    Some(name) => severity(&name)?,
                    None => current.severity,
                },
            },
        );
    }

    Ok(Settings {
        policy,
        exclude: document.exclude,
    })
}

fn severity(name: &str) -> Result<Severity> {
    match name {
        "off" => Ok(Severity::Off),
        "warning" => Ok(Severity::Warning),
        "error" => Ok(Severity::Error),
        other => bail!("unknown severity {other}, expected off, warning or error"),
    }
}
