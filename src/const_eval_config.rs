use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use std::fs;

#[derive(Debug, Clone, Copy)]
pub struct ConstEvalConfig {
    pub fuel_limit: Option<usize>,
    pub enable_expression_memo: bool,
}

impl ConstEvalConfig {
    #[must_use]
    pub fn with_fuel_limit(mut self, limit: Option<usize>) -> Self {
        self.fuel_limit = limit;
        self
    }
}

impl Default for ConstEvalConfig {
    fn default() -> Self {
        Self {
            fuel_limit: Some(10_000),
            enable_expression_memo: true,
        }
    }
}

static GLOBAL_CONFIG: OnceLock<RwLock<ConstEvalConfig>> = OnceLock::new();

fn config_cell() -> &'static RwLock<ConstEvalConfig> {
    GLOBAL_CONFIG.get_or_init(|| RwLock::new(ConstEvalConfig::default()))
}

/// Update the process-wide const-eval configuration.
pub fn set_global(config: ConstEvalConfig) {
    if let Ok(mut guard) = config_cell().write() {
        *guard = config;
    }
}

/// Retrieve the current const-eval configuration.
#[must_use]
pub fn current() -> ConstEvalConfig {
    config_cell().read().map(|guard| *guard).unwrap_or_default()
}

/// Resolve a const-eval configuration by merging file defaults with a CLI override.
#[must_use]
pub fn resolve(cli_override: Option<usize>) -> ConstEvalConfig {
    let mut config = load_from_config_file().unwrap_or_default();
    if let Some(limit) = cli_override {
        config = config.with_fuel_limit(Some(limit));
    }
    config
}

fn load_from_config_file() -> Option<ConstEvalConfig> {
    const PATH_CANDIDATES: &[fn() -> Option<PathBuf>] = &[
        || std::env::current_dir().ok().map(|dir| dir.join("chic.cfg")),
        || Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("chic.cfg")),
    ];

    for candidate in PATH_CANDIDATES {
        if let Some(path) = candidate() {
            if path.exists() {
                if let Ok(contents) = fs::read_to_string(&path) {
                    if let Some(config) = parse_config(&contents) {
                        return Some(config);
                    }
                }
            }
        }
    }
    None
}

fn parse_config(contents: &str) -> Option<ConstEvalConfig> {
    let mut config = ConstEvalConfig::default();
    let mut seen = false;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.splitn(2, '=');
        let key = parts.next()?.trim();
        let value = parts.next()?.trim();
        match key {
            "consteval.fuel_limit" | "consteval_fuel_limit" => {
                if let Ok(parsed) = value.parse::<usize>() {
                    config.fuel_limit = Some(parsed);
                    seen = true;
                }
            }
            "consteval.enable_memo" | "consteval_memo" => match value {
                "true" | "1" | "yes" => {
                    config.enable_expression_memo = true;
                    seen = true;
                }
                "false" | "0" | "no" => {
                    config.enable_expression_memo = false;
                    seen = true;
                }
                _ => {}
            },
            _ => {}
        }
    }

    if seen { Some(config) } else { None }
}
