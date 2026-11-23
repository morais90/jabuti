use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use jabuti_core::catalog::Concept;
use jabuti_core::lang::LanguageId;
use jabuti_core::policy::{ConceptBindings, Policy};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Layer {
    pub(crate) name: String,
    pub(crate) paths: Vec<String>,
    pub(crate) depends_on: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    #[serde(default)]
    exclude: Vec<String>,
    rules: Option<toml::Value>,
    #[serde(default)]
    tools: BTreeMap<String, ToolEntry>,
    #[serde(default)]
    languages: BTreeMap<String, LanguageEntry>,
    #[serde(default)]
    layers: BTreeMap<String, LayerEntry>,
    coverage: Option<CoverageEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoverageEntry {
    report: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LayerEntry {
    paths: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LanguageEntry {
    rules: Option<toml::Value>,
    #[serde(default)]
    concepts: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolEntry {
    #[serde(default)]
    enabled: bool,
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

const FIXED: &str = "jabuti fixes which rules run and their limits, and every finding it reports is one to fix, so there is nothing to set there. Remove the table";

fn language_settings(
    bindings: &mut ConceptBindings,
    name: &str,
    entry: LanguageEntry,
) -> Result<()> {
    let language =
        LanguageId::from_name(name).with_context(|| format!("unknown language {name}"))?;
    if entry.rules.is_some() {
        bail!("[languages.{name}.rules] is not read: {FIXED}");
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
    if document.rules.is_some() {
        bail!("[rules] is not read: {FIXED}");
    }
    let mut concepts = ConceptBindings::default();

    for (name, entry) in document.languages {
        language_settings(&mut concepts, &name, entry)?;
    }

    Ok(Settings {
        policy: Policy::default(),
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
