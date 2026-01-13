use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodeStyleEnforcement {
    Off,
    Warn,
    Error,
}

impl CodeStyleEnforcement {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" | "none" | "disable" => Some(Self::Off),
            "warn" | "warning" => Some(Self::Warn),
            "error" | "err" => Some(Self::Error),
            _ => None,
        }
    }
}

impl Default for CodeStyleEnforcement {
    fn default() -> Self {
        Self::Warn
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeStyleConfig {
    pub version: u32,
    pub enforce: CodeStyleEnforcement,
    pub profile_default: String,
    pub profiles: HashMap<String, CleanupProfile>,
    pub rules: CodeStyleRules,
    pub import: CodeStyleImportConfig,
}

impl CodeStyleConfig {
    #[must_use]
    pub fn default_for_env(is_ci: bool) -> Self {
        Self {
            version: 1,
            enforce: if is_ci {
                CodeStyleEnforcement::Error
            } else {
                CodeStyleEnforcement::Warn
            },
            profile_default: "default".to_string(),
            profiles: HashMap::from([(
                "default".to_string(),
                CleanupProfile {
                    actions: vec![
                        "formatting".to_string(),
                        "syntax_style".to_string(),
                        "imports".to_string(),
                        "layout".to_string(),
                    ],
                    unsafe_actions: Vec::new(),
                },
            )]),
            rules: CodeStyleRules::default(),
            import: CodeStyleImportConfig::default(),
        }
    }

    #[must_use]
    pub fn from_raw(raw: &RawCodeStyleSection) -> Self {
        let mut config = Self::default_for_env(is_ci_environment());
        if let Some(version) = raw.version {
            config.version = version;
        }
        if let Some(enforce) = raw.enforce.as_deref().and_then(CodeStyleEnforcement::parse) {
            config.enforce = enforce;
        }
        if let Some(default_profile) = &raw.profile_default {
            config.profile_default = default_profile.clone();
        }
        for (name, profile) in &raw.profiles {
            config
                .profiles
                .insert(name.clone(), CleanupProfile::from_raw(profile));
        }
        config.rules = CodeStyleRules::from_raw(&raw.rules);
        config.import = CodeStyleImportConfig::from_raw(&raw.import);
        config
    }
}

fn is_ci_environment() -> bool {
    std::env::var("CI").is_ok() || std::env::var("TF_BUILD").is_ok()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupProfile {
    pub actions: Vec<String>,
    pub unsafe_actions: Vec<String>,
}

impl CleanupProfile {
    fn from_raw(raw: &RawCleanupProfile) -> Self {
        let mut profile = Self {
            actions: raw.actions.clone().unwrap_or_else(|| {
                vec![
                    "formatting".to_string(),
                    "syntax_style".to_string(),
                    "imports".to_string(),
                    "layout".to_string(),
                ]
            }),
            unsafe_actions: raw.unsafe_actions.clone().unwrap_or_else(|| Vec::new()),
        };
        profile.actions.retain(|action| !action.is_empty());
        profile.unsafe_actions.retain(|action| !action.is_empty());
        profile
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CodeStyleRules {
    #[serde(default)]
    pub braces: BraceRules,
    #[serde(default)]
    pub wrap: WrapRules,
    #[serde(default)]
    pub files: FileRules,
    #[serde(default)]
    pub usings: UsingRules,
    #[serde(default)]
    pub naming: NamingRules,
    #[serde(default)]
    pub raw: std::collections::HashMap<String, serde_yaml::Value>,
}

impl CodeStyleRules {
    #[must_use]
    pub fn from_raw(raw: &RawRules) -> Self {
        let mut rules = Self::default();
        if let Some(req) = raw.braces.if_else {
            rules.braces.if_else = Some(req);
        }
        if let Some(req) = raw.braces.r#for {
            rules.braces.r#for = Some(req);
        }
        if let Some(style) = raw.wrap.arguments {
            rules.wrap.arguments = Some(style);
        }
        if let Some(style) = raw.wrap.parameters {
            rules.wrap.parameters = Some(style);
        }
        if let Some(trailing) = raw.files.trailing_newline {
            rules.files.trailing_newline = Some(trailing);
        }
        if let Some(blank) = raw.usings.blank_lines_between_groups {
            rules.usings.blank_lines_between_groups = Some(blank);
        }
        if let Some(abbrevs) = raw.naming.abbreviations.clone() {
            rules.naming.abbreviations = abbrevs;
        }
        if let Some(policies) = raw.naming.raw_policies.clone() {
            rules.naming.raw_policies = policies;
        }
        if let Some(words) = raw.naming.dictionary_words.clone() {
            rules.naming.dictionary_words = words;
        }
        if let Some(extra) = raw.raw.clone() {
            rules.raw = extra;
        }
        rules
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BraceRules {
    #[serde(rename = "if_else")]
    pub if_else: Option<BraceRequirement>,
    #[serde(rename = "for")]
    pub r#for: Option<BraceRequirement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BraceRequirement {
    Required,
    Optional,
    Omit,
}

impl BraceRequirement {
    #[must_use]
    pub fn from_dotsettings(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "required" | "true" => Some(Self::Required),
            "optional" | "false" => Some(Self::Optional),
            "omit" | "none" => Some(Self::Omit),
            _ => None,
        }
    }
}

impl Default for CodeStyleConfig {
    fn default() -> Self {
        Self::default_for_env(is_ci_environment())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WrapRules {
    pub arguments: Option<WrapStyle>,
    pub parameters: Option<WrapStyle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WrapStyle {
    ChopIfLong,
    None,
    WrapAlways,
}

impl WrapStyle {
    #[must_use]
    pub fn from_dotsettings(value: &str) -> Option<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "CHOP_IF_LONG" => Some(Self::ChopIfLong),
            "WRAP_ALWAYS" | "WRAP_ALWAYS_IDENT" => Some(Self::WrapAlways),
            "OFF" | "NONE" => Some(Self::None),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FileRules {
    pub trailing_newline: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct UsingRules {
    pub blank_lines_between_groups: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NamingRules {
    #[serde(default)]
    pub abbreviations: Vec<String>,
    #[serde(default)]
    pub raw_policies: Vec<NamingPolicy>,
    #[serde(default)]
    pub dictionary_words: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamingPolicy {
    pub name: String,
    #[serde(default)]
    pub inspect: bool,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub suffix: Option<String>,
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub descriptor: Option<NamingDescriptor>,
    #[serde(default)]
    pub extra_rules: Vec<ExtraRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NamingDescriptor {
    #[serde(default)]
    pub staticness: Option<String>,
    #[serde(default)]
    pub access_right_kinds: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub element_kinds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ExtraRule {
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub suffix: Option<String>,
    #[serde(default)]
    pub style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeStyleImportConfig {
    #[serde(default)]
    pub dotsettings_paths: Vec<String>,
    #[serde(default)]
    pub dotsettings_mode: DotSettingsMergeMode,
    #[serde(default)]
    pub dotsettings_unknown_keys: UnknownKeyPolicy,
    #[serde(default)]
    pub unmapped: Vec<UnmappedSetting>,
}

impl Default for CodeStyleImportConfig {
    fn default() -> Self {
        Self {
            dotsettings_paths: Vec::new(),
            dotsettings_mode: DotSettingsMergeMode::Merge,
            dotsettings_unknown_keys: UnknownKeyPolicy::Preserve,
            unmapped: Vec::new(),
        }
    }
}

impl CodeStyleImportConfig {
    fn from_raw(raw: &RawImportSection) -> Self {
        Self {
            dotsettings_paths: raw.dotsettings_paths.clone().unwrap_or_default(),
            dotsettings_mode: raw.dotsettings_mode.unwrap_or(DotSettingsMergeMode::Merge),
            dotsettings_unknown_keys: raw
                .dotsettings_unknown_keys
                .unwrap_or(UnknownKeyPolicy::Preserve),
            unmapped: raw.unmapped.clone().unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DotSettingsMergeMode {
    Merge,
    Replace,
}

impl Default for DotSettingsMergeMode {
    fn default() -> Self {
        Self::Merge
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnknownKeyPolicy {
    Preserve,
    Ignore,
}

impl Default for UnknownKeyPolicy {
    fn default() -> Self {
        Self::Preserve
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnmappedSetting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawCodeStyleSection {
    #[serde(default)]
    pub version: Option<u32>,
    #[serde(default)]
    pub enforce: Option<String>,
    #[serde(default)]
    pub profile_default: Option<String>,
    #[serde(default)]
    pub profiles: HashMap<String, RawCleanupProfile>,
    #[serde(default)]
    pub rules: RawRules,
    #[serde(default)]
    pub import: RawImportSection,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawCleanupProfile {
    #[serde(default)]
    pub actions: Option<Vec<String>>,
    #[serde(default)]
    pub unsafe_actions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawRules {
    #[serde(default)]
    pub braces: RawBraces,
    #[serde(default)]
    pub wrap: RawWrap,
    #[serde(default)]
    pub files: RawFiles,
    #[serde(default)]
    pub usings: RawUsings,
    #[serde(default)]
    pub naming: RawNaming,
    #[serde(default)]
    pub raw: Option<std::collections::HashMap<String, serde_yaml::Value>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawBraces {
    #[serde(default, rename = "if_else")]
    pub if_else: Option<BraceRequirement>,
    #[serde(default, rename = "for")]
    pub r#for: Option<BraceRequirement>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawWrap {
    #[serde(default)]
    pub arguments: Option<WrapStyle>,
    #[serde(default)]
    pub parameters: Option<WrapStyle>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawFiles {
    #[serde(default)]
    pub trailing_newline: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawUsings {
    #[serde(default)]
    pub blank_lines_between_groups: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawNaming {
    #[serde(default)]
    pub abbreviations: Option<Vec<String>>,
    #[serde(default)]
    pub raw_policies: Option<Vec<NamingPolicy>>,
    #[serde(default)]
    pub dictionary_words: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawImportSection {
    #[serde(default, rename = "dotsettings_paths")]
    pub dotsettings_paths: Option<Vec<String>>,
    #[serde(default, rename = "dotsettings_mode")]
    pub dotsettings_mode: Option<DotSettingsMergeMode>,
    #[serde(default, rename = "dotsettings_unknown_keys")]
    pub dotsettings_unknown_keys: Option<UnknownKeyPolicy>,
    #[serde(default)]
    pub unmapped: Option<Vec<UnmappedSetting>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_style_defaults_match_ci_env() {
        let config = CodeStyleConfig::default_for_env(true);
        assert_eq!(config.enforce, CodeStyleEnforcement::Error);
        assert!(config.profiles.contains_key("default"));
    }

    #[test]
    fn raw_section_overrides_defaults() {
        let mut raw = RawCodeStyleSection::default();
        raw.enforce = Some("off".into());
        raw.profile_default = Some("custom".into());
        raw.profiles.insert(
            "custom".into(),
            RawCleanupProfile {
                actions: Some(vec!["formatting".into()]),
                unsafe_actions: None,
            },
        );
        raw.rules.files.trailing_newline = Some(false);
        let config = CodeStyleConfig::from_raw(&raw);
        assert_eq!(config.enforce, CodeStyleEnforcement::Off);
        assert_eq!(config.profile_default, "custom");
        assert_eq!(
            config.rules.files.trailing_newline,
            Some(false),
            "file rules should propagate"
        );
    }
}
