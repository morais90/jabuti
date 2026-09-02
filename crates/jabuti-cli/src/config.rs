use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use jabuti_core::lang::LanguageId;
use jabuti_core::model::{RuleId, Severity};
use jabuti_core::policy::{Policy, RuleConfig};
use serde::Deserialize;

pub(crate) const FILE_NAME: &str = "jabuti.toml";

#[derive(Debug, Default)]
pub(crate) struct Settings {
    pub(crate) policy: Policy,
    pub(crate) exclude: Vec<String>,
    pub(crate) tools: BTreeMap<String, bool>,
    pub(crate) layers: Vec<Layer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Layer {
    pub(crate) name: String,
    pub(crate) paths: Vec<String>,
    pub(crate) depends_on: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Document {
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    rules: BTreeMap<String, Entry>,
    #[serde(default)]
    tools: BTreeMap<String, ToolEntry>,
    #[serde(default)]
    languages: BTreeMap<String, LanguageEntry>,
    #[serde(default)]
    layers: BTreeMap<String, LayerEntry>,
}

#[derive(Debug, Deserialize)]
struct LayerEntry {
    paths: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct LanguageEntry {
    #[serde(default)]
    rules: BTreeMap<String, Entry>,
}

#[derive(Debug, Deserialize)]
struct ToolEntry {
    #[serde(default)]
    enabled: bool,
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

fn language_rules(policy: &mut Policy, name: &str, entry: LanguageEntry) -> Result<()> {
    let language =
        LanguageId::from_name(name).with_context(|| format!("unknown language {name}"))?;

    for (id, rule) in entry.rules {
        let target = RuleId::parse(&id).with_context(|| format!("unknown rule {id}"))?;
        if matches!(&target, RuleId::Native(native) if native.repository_wide()) {
            bail!("{id} is measured across the whole repository, so it cannot be set per language");
        }

        let current = policy
            .config_for(language, target.clone())
            .unwrap_or(RuleConfig {
                limit: 0,
                severity: Severity::Warning,
            });

        policy.set_for(language, target, adjusted(current, rule)?);
    }

    Ok(())
}

fn settings(document: Document) -> Result<Settings> {
    let mut policy = Policy::default();

    for (id, entry) in document.rules {
        let rule = RuleId::parse(&id).with_context(|| format!("unknown rule {id}"))?;
        let current = policy.config(rule.clone()).unwrap_or(RuleConfig {
            limit: 0,
            severity: Severity::Warning,
        });

        policy.set(rule, adjusted(current, entry)?);
    }

    for (name, entry) in document.languages {
        language_rules(&mut policy, &name, entry)?;
    }

    let known: Vec<&str> = crate::tools::ALL.iter().map(|tool| tool.name).collect();
    for name in document.tools.keys() {
        if !known.contains(&name.as_str()) {
            bail!("unknown tool {name}, jabuti knows {}", known.join(", "));
        }
    }

    Ok(Settings {
        policy,
        exclude: document.exclude,
        tools: document
            .tools
            .into_iter()
            .map(|(name, entry)| (name, entry.enabled))
            .collect(),
        layers: layers(document.layers)?,
    })
}

fn layers(declared: BTreeMap<String, LayerEntry>) -> Result<Vec<Layer>> {
    let names: Vec<&str> = declared.keys().map(String::as_str).collect();

    for (name, entry) in &declared {
        checked(name, entry, &names)?;
    }

    Ok(declared
        .into_iter()
        .map(|(name, entry)| Layer {
            name,
            paths: entry.paths,
            depends_on: entry.depends_on,
        })
        .collect())
}

fn checked(name: &str, entry: &LayerEntry, names: &[&str]) -> Result<()> {
    if entry.paths.is_empty() {
        bail!("layer {name} names no paths");
    }

    for target in &entry.depends_on {
        if !names.contains(&target.as_str()) {
            bail!(
                "layer {name} depends on {target}, which is not a declared layer (declared: {})",
                names.join(", ")
            );
        }
    }

    Ok(())
}

fn adjusted(current: RuleConfig, entry: Entry) -> Result<RuleConfig> {
    Ok(RuleConfig {
        limit: entry.limit.unwrap_or(current.limit),
        severity: match entry.severity {
            Some(name) => severity(&name)?,
            None => current.severity,
        },
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
