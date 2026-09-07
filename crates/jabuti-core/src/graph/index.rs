use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use super::facts::FileFacts;
use crate::lang::LanguageId;
use crate::model::Span;

#[derive(Debug, Clone)]
pub struct Source {
    pub path: PathBuf,
    pub language: LanguageId,
    pub facts: FileFacts,
}

pub type Edges = BTreeMap<(PathBuf, PathBuf), Span>;

pub fn edges(sources: &[Source]) -> Edges {
    let index = Index::of(sources);
    let mut found = Edges::new();

    for source in sources {
        for (target, at) in index.targets(source) {
            if target != source.path {
                found.insert((source.path.clone(), target), at);
            }
        }
    }

    found
}

#[derive(Debug, Default)]
pub struct Index {
    modules: BTreeMap<(PathBuf, Vec<String>), PathBuf>,
    crates: BTreeMap<(String, Vec<String>), PathBuf>,
    declarations: BTreeMap<(String, String), PathBuf>,
    typescript_modules: BTreeMap<PathBuf, PathBuf>,
}

impl Index {
    pub fn of(sources: &[Source]) -> Self {
        let mut index = Self::default();

        for source in sources {
            match source.language {
                LanguageId::Rust => index.add_module(source),
                LanguageId::Kotlin => index.add_declarations(source),
                LanguageId::TypeScript => index.add_typescript_module(source),
            }
        }

        index
    }

    fn add_module(&mut self, source: &Source) {
        let (root, segments) = rust_module(&source.path);

        if let Some(name) = crate_name(&root) {
            self.crates
                .insert((name, segments.clone()), source.path.clone());
        }

        self.modules.insert((root, segments), source.path.clone());
    }

    fn add_declarations(&mut self, source: &Source) {
        for name in source.facts.declares.keys() {
            let key = (source.facts.module.clone(), name.clone());
            self.declarations.insert(key, source.path.clone());
        }
    }

    fn add_typescript_module(&mut self, source: &Source) {
        let mut module = source.path.clone();
        module.set_extension("");
        self.add_typescript_key(&module, &source.path);

        if module.extension().is_some_and(|extension| extension == "d") {
            module.set_extension("");
            self.add_typescript_key(&module, &source.path);
        }
    }

    fn add_typescript_key(&mut self, module: &Path, source: &Path) {
        self.typescript_modules
            .insert(module.to_path_buf(), source.to_path_buf());

        if module.file_name().is_some_and(|name| name == "index")
            && let Some(directory) = module.parent()
        {
            self.typescript_modules
                .insert(directory.to_path_buf(), source.to_path_buf());
        }
    }

    pub fn targets(&self, source: &Source) -> BTreeMap<PathBuf, Span> {
        let mut reached = match source.language {
            LanguageId::Rust => rust_targets(source, &self.modules, &self.crates),
            LanguageId::Kotlin => kotlin_targets(source, &self.declarations),
            LanguageId::TypeScript => typescript_targets(source, &self.typescript_modules),
        };
        reached.sort_by_key(|(target, at)| (target.clone(), at.start_line));

        let mut earliest: BTreeMap<PathBuf, Span> = BTreeMap::new();
        for (target, at) in reached {
            earliest.entry(target).or_insert(at);
        }

        earliest
    }

    pub fn descendant_module(&self, of: &Path, names: &[&str]) -> Option<PathBuf> {
        let (root, mut segments) = rust_module(of);
        segments.extend(names.iter().map(|name| (*name).to_owned()));

        self.modules.get(&(root, segments)).cloned()
    }
}

fn rust_module(path: &Path) -> (PathBuf, Vec<String>) {
    let components: Vec<String> = path
        .components()
        .map(|part| part.as_os_str().to_string_lossy().to_string())
        .collect();

    let boundary = components.iter().rposition(|part| part == "src");
    let (root, rest) = match boundary {
        Some(index) => (
            components[..=index].iter().collect::<PathBuf>(),
            &components[index + 1..],
        ),
        None => (
            path.parent().unwrap_or(Path::new("")).to_path_buf(),
            components
                .last()
                .map_or(&components[..0], std::slice::from_ref),
        ),
    };

    let mut segments: Vec<String> = rest.iter().map(|part| stem(part)).collect();
    if segments
        .last()
        .is_some_and(|last| last == "mod" || last == "lib" || last == "main")
    {
        segments.pop();
    }

    (root, segments)
}

fn stem(component: &str) -> String {
    component
        .strip_suffix(".rs")
        .unwrap_or(component)
        .to_owned()
}

fn rust_targets(
    source: &Source,
    modules: &BTreeMap<(PathBuf, Vec<String>), PathBuf>,
    crates: &BTreeMap<(String, Vec<String>), PathBuf>,
) -> Vec<(PathBuf, Span)> {
    let (root, here) = rust_module(&source.path);
    let mut found = Vec::new();

    for (path, at) in &source.facts.paths {
        let segments: Vec<&str> = path.split("::").collect();
        let inside = anchored(&segments, &here)
            .and_then(|absolute| longest(modules, &root, absolute))
            .filter(|target| target != &source.path);

        if let Some(target) = inside.or_else(|| in_another_crate(&segments, crates)) {
            found.push((target, *at));
        }
    }

    found
}

fn crate_name(root: &Path) -> Option<String> {
    if root.file_name()? != "src" {
        return None;
    }

    let directory = root.parent()?.file_name()?.to_string_lossy().to_string();

    Some(directory.replace('-', "_"))
}

fn in_another_crate(
    segments: &[&str],
    crates: &BTreeMap<(String, Vec<String>), PathBuf>,
) -> Option<PathBuf> {
    let (first, rest) = segments.split_first()?;
    let name = (*first).to_owned();
    let mut remaining: Vec<String> = rest.iter().map(|part| (*part).to_owned()).collect();

    loop {
        if let Some(target) = crates.get(&(name.clone(), remaining.clone())) {
            return Some(target.clone());
        }
        if remaining.is_empty() {
            return None;
        }
        remaining.pop();
    }
}

fn anchored(segments: &[&str], here: &[String]) -> Option<Vec<String>> {
    let (first, rest) = segments.split_first()?;

    let mut absolute = match *first {
        "crate" => Vec::new(),
        "self" => here.to_vec(),
        "super" => here.iter().rev().skip(1).rev().cloned().collect(),
        name => {
            let mut child = here.to_vec();
            child.push(name.to_owned());
            child
        }
    };
    absolute.extend(rest.iter().map(|part| (*part).to_owned()));

    Some(absolute)
}

fn longest(
    modules: &BTreeMap<(PathBuf, Vec<String>), PathBuf>,
    root: &Path,
    mut segments: Vec<String>,
) -> Option<PathBuf> {
    loop {
        if let Some(target) = modules.get(&(root.to_path_buf(), segments.clone())) {
            return Some(target.clone());
        }
        segments.pop()?;
    }
}

fn kotlin_targets(
    source: &Source,
    declarations: &BTreeMap<(String, String), PathBuf>,
) -> Vec<(PathBuf, Span)> {
    let mut found = Vec::new();

    for (path, at) in &source.facts.paths {
        if let Some((package, name)) = path.rsplit_once('.')
            && let Some(target) = declarations.get(&(package.to_owned(), name.to_owned()))
        {
            found.push((target.clone(), *at));
        }
    }

    for (name, spans) in &source.facts.names {
        let key = (source.facts.module.clone(), name.clone());
        if let (Some(target), Some(at)) = (declarations.get(&key), spans.first()) {
            found.push((target.clone(), *at));
        }
    }

    found
}

fn typescript_targets(
    source: &Source,
    modules: &BTreeMap<PathBuf, PathBuf>,
) -> Vec<(PathBuf, Span)> {
    source
        .facts
        .paths
        .iter()
        .filter_map(|(reference, at)| {
            let relative = reference.strip_prefix('.')?;
            let parent = source.path.parent().unwrap_or(Path::new(""));
            let mut module = normalised(&parent.join(format!(".{relative}")));
            if matches!(
                module.extension().and_then(|extension| extension.to_str()),
                Some("ts" | "tsx" | "js" | "jsx" | "mts" | "cts" | "mjs" | "cjs")
            ) {
                module.set_extension("");
            }
            modules.get(&module).cloned().map(|target| (target, *at))
        })
        .collect()
}

fn normalised(path: &Path) -> PathBuf {
    let mut normal = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normal.pop();
            }
            Component::Normal(part) => normal.push(part),
            Component::RootDir | Component::Prefix(_) => normal.push(component.as_os_str()),
        }
    }

    normal
}
