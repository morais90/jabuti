use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use jabuti_core::catalog::{Concept, Input, Rule, RuleId, Severity};
use jabuti_core::lang::LanguageId;
use jabuti_core::policy::{ConceptBindings, Policy, RuleConfig};
use serde::Deserialize;

pub(crate) const FILE_NAME: &str = "jabuti.toml";

#[derive(Debug, Default)]
pub(crate) struct Settings {
    pub(crate) policy: Policy,
    pub(crate) concepts: ConceptBindings,
    pub(crate) exclude: Vec<String>,
    pub(crate) tools: BTreeMap<String, bool>,
    pub(crate) layers: Vec<Layer>,
    pub(crate) coverage: Option<PathBuf>,
}

impl Settings {
    pub(crate) fn needs(&self, input: Input) -> bool {
        Rule::ALL
            .into_iter()
            .any(|rule| self.policy.enabled(rule) && rule.spec().inputs.contains(&input))
    }
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
    coverage: Option<CoverageEntry>,
}

#[derive(Debug, Deserialize)]
struct CoverageEntry {
    report: PathBuf,
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
    #[serde(default)]
    concepts: BTreeMap<String, Vec<String>>,
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

fn language_settings(
    policy: &mut Policy,
    bindings: &mut ConceptBindings,
    name: &str,
    entry: LanguageEntry,
) -> Result<()> {
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

        policy.set_for(language, target, Override::of(&rule)?.applied_to(current));
    }

    for (id, paths) in entry.concepts {
        let concept = Concept::from_id(&id).with_context(|| format!("unknown concept {id}"))?;
        if paths.is_empty() || paths.iter().any(String::is_empty) {
            bail!("concept {id} must name at least one non-empty API path");
        }
        bindings.set(language, concept, paths);
    }

    Ok(())
}

fn settings(document: Document) -> Result<Settings> {
    let mut policy = Policy::default();
    let mut concepts = ConceptBindings::default();

    for (id, entry) in document.rules {
        let rule = RuleId::parse(&id).with_context(|| format!("unknown rule {id}"))?;
        let set = Override::of(&entry)?;
        let current = policy.config(rule.clone()).unwrap_or(RuleConfig {
            limit: 0,
            severity: Severity::Warning,
        });

        policy.set(rule.clone(), set.applied_to(current));
        policy.adjust_per_language(rule, |config| set.applied_to(config));
    }

    for (name, entry) in document.languages {
        language_settings(&mut policy, &mut concepts, &name, entry)?;
    }

    Ok(Settings {
        policy,
        concepts,
        exclude: document.exclude,
        tools: document
            .tools
            .into_iter()
            .map(|(name, entry)| (name, entry.enabled))
            .collect(),
        layers: layers(document.layers)?,
        coverage: document.coverage.map(|entry| entry.report),
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

#[derive(Debug, Clone, Copy)]
struct Override {
    limit: Option<u32>,
    severity: Option<Severity>,
}

impl Override {
    fn of(entry: &Entry) -> Result<Self> {
        Ok(Self {
            limit: entry.limit,
            severity: entry.severity.as_deref().map(severity).transpose()?,
        })
    }

    fn applied_to(self, current: RuleConfig) -> RuleConfig {
        RuleConfig {
            limit: self.limit.unwrap_or(current.limit),
            severity: self.severity.unwrap_or(current.severity),
        }
    }
}

fn severity(name: &str) -> Result<Severity> {
    match name {
        "off" => Ok(Severity::Off),
        "warning" => Ok(Severity::Warning),
        "error" => Ok(Severity::Error),
        other => bail!("unknown severity {other}, expected off, warning or error"),
    }
}
