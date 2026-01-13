use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};

use super::canonical_lint_name;
use super::diagnostic::{LintCategory, LintLevel};

#[derive(Debug, Clone)]
pub struct LintConfig {
    category_levels: HashMap<LintCategory, LintLevel>,
    rule_levels: HashMap<String, LintLevel>,
}

impl Default for LintConfig {
    fn default() -> Self {
        let mut category_levels = HashMap::new();
        category_levels.insert(LintCategory::Style, LintLevel::Warn);
        category_levels.insert(LintCategory::Correctness, LintLevel::Error);
        category_levels.insert(LintCategory::Perf, LintLevel::Warn);
        category_levels.insert(LintCategory::Pedantic, LintLevel::Allow);
        let mut rule_levels = HashMap::new();
        rule_levels.insert(canonical_lint_name("dead_code"), LintLevel::Warn);
        Self {
            category_levels,
            rule_levels,
        }
    }
}

impl LintConfig {
    #[must_use]
    pub fn level_for(&self, name: &str, category: LintCategory, default: LintLevel) -> LintLevel {
        let canonical = canonical_lint_name(name);
        if let Some(level) = self.rule_levels.get(&canonical) {
            return *level;
        }
        if let Some(level) = self.category_levels.get(&category) {
            *level
        } else {
            default
        }
    }

    pub fn apply_layer(&mut self, layer: LintConfigLayer) {
        for (category, level) in layer.category_levels {
            self.category_levels.insert(category, level);
        }
        for (rule, level) in layer.rule_levels {
            self.rule_levels.insert(rule, level);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LintConfigLayer {
    pub category_levels: HashMap<LintCategory, LintLevel>,
    pub rule_levels: HashMap<String, LintLevel>,
}

#[derive(Debug, Deserialize, Default)]
struct RawLintDocument {
    #[serde(default)]
    extends: Option<String>,
    #[serde(default)]
    #[serde(alias = "category-levels")]
    categories: HashMap<String, String>,
    #[serde(default)]
    rules: HashMap<String, String>,
}

fn parse_layer(path: &Path, visited: &mut HashSet<PathBuf>) -> Result<Vec<LintConfigLayer>> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !visited.insert(canonical.clone()) {
        return Err(Error::internal(format!(
            "cyclic lint configuration detected involving {}",
            path.display()
        )));
    }
    let mut contents = fs::read_to_string(path).map_err(|err| {
        Error::internal(format!(
            "failed to read lint config `{}`: {err}",
            path.display()
        ))
    })?;
    if contents.starts_with("---\n") {
        // serde_yaml tolerates the header, but normalise to keep error messages clean.
        contents = contents.replacen("---\n", "", 1);
    }
    let yaml: serde_yaml::Value = serde_yaml::from_str(&contents).map_err(|err| {
        Error::internal(format!(
            "failed to parse lint config `{}`: {err}",
            path.display()
        ))
    })?;
    let section = match yaml {
        serde_yaml::Value::Mapping(ref mapping)
            if mapping.contains_key(&serde_yaml::Value::String("lint".into())) =>
        {
            mapping
                .get(&serde_yaml::Value::String("lint".into()))
                .cloned()
        }
        serde_yaml::Value::Mapping(_) => Some(yaml.clone()),
        _ => None,
    };
    let Some(section) = section else {
        return Ok(Vec::new());
    };
    let raw: RawLintDocument = serde_yaml::from_value(section).map_err(|err| {
        Error::internal(format!(
            "failed to parse lint settings in `{}`: {err}",
            path.display()
        ))
    })?;

    let mut layers = Vec::new();
    if let Some(extends) = raw.extends.as_deref() {
        let base = path
            .parent()
            .map(|parent| parent.join(extends))
            .unwrap_or_else(|| PathBuf::from(extends));
        layers.extend(parse_layer(&base, visited)?);
    }
    layers.push(raw.into_layer(path));
    Ok(layers)
}

impl RawLintDocument {
    fn into_layer(self, path: &Path) -> LintConfigLayer {
        let mut layer = LintConfigLayer::default();
        for (category, level) in self.categories {
            if let (Some(cat), Some(level)) = (
                LintCategory::from_str(&category),
                LintLevel::from_str(&level),
            ) {
                layer.category_levels.insert(cat, level);
            } else {
                tracing::warn!(
                    target: "lint-config",
                    "ignored category entry `{category}` -> `{level}` in {}",
                    path.display()
                );
            }
        }
        for (rule, level) in self.rules {
            if let Some(level) = LintLevel::from_str(&level) {
                layer.rule_levels.insert(canonical_lint_name(&rule), level);
            } else {
                tracing::warn!(
                    target: "lint-config",
                    "ignored rule entry `{rule}` -> `{level}` in {}",
                    path.display()
                );
            }
        }
        layer
    }
}

fn discover_layers(start_dir: &Path) -> Result<Vec<LintConfigLayer>> {
    let mut cursor = Some(start_dir.to_path_buf());
    let mut discovered = Vec::new();
    let mut visited = HashSet::new();
    while let Some(dir) = cursor {
        for candidate in [
            "lint.yaml",
            "chiclint.yaml",
            "lint.yml",
            crate::manifest::PROJECT_MANIFEST_BASENAME,
        ] {
            let path = dir.join(candidate);
            if path.exists() && path.is_file() {
                let mut layers = parse_layer(&path, &mut visited)?;
                discovered.append(&mut layers);
            }
        }
        cursor = dir.parent().map(Path::to_path_buf);
    }
    // Walked from leaf to root; apply root-most first.
    discovered.reverse();
    Ok(discovered)
}

fn start_dir_for_inputs(inputs: &[PathBuf]) -> Result<PathBuf> {
    if let Some(first) = inputs.first() {
        if first.is_dir() {
            Ok(first.clone())
        } else {
            first
                .parent()
                .map(Path::to_path_buf)
                .ok_or_else(|| Error::internal("input had no parent directory"))
        }
    } else {
        std::env::current_dir().map_err(|err| Error::internal(err.to_string()))
    }
}

pub fn discover(inputs: &[PathBuf]) -> Result<LintConfig> {
    discover_with_override(inputs, None)
}

pub fn discover_with_override(inputs: &[PathBuf], explicit: Option<&Path>) -> Result<LintConfig> {
    let mut config = LintConfig::default();
    let start_dir = start_dir_for_inputs(inputs)?;
    for layer in discover_layers(&start_dir)? {
        config.apply_layer(layer);
    }

    if let Some(path) = explicit {
        for layer in parse_layer(path, &mut HashSet::new())? {
            config.apply_layer(layer);
        }
    }

    if let Ok(env_path) = std::env::var("CHIC_LINT_CONFIG") {
        let env_path = PathBuf::from(env_path);
        if env_path.exists() {
            for layer in parse_layer(&env_path, &mut HashSet::new())? {
                config.apply_layer(layer);
            }
        }
    }

    Ok(config)
}
