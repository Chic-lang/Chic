use serde::Deserialize;
use std::env;

/// Formatter configuration loaded from `manifest.yaml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatConfig {
    pub version: u32,
    pub enabled: bool,
    pub enforce: FormatEnforcement,
    pub max_line_length: usize,
    pub indent: IndentStyle,
    pub newline: NewlineStyle,
    pub trailing_newline: bool,
    pub trim_trailing_whitespace: bool,
    pub braces: BraceSettings,
    pub r#if: IfSettings,
    pub switch: SwitchSettings,
    pub usings: UsingSettings,
    pub ordering: OrderingRules,
    pub files: FileOrganization,
}

impl FormatConfig {
    #[must_use]
    pub fn default_for_env(is_ci: bool) -> Self {
        Self {
            version: 1,
            enabled: true,
            enforce: if is_ci {
                FormatEnforcement::Error
            } else {
                FormatEnforcement::Warn
            },
            max_line_length: 120,
            indent: IndentStyle::default(),
            newline: NewlineStyle::default(),
            trailing_newline: true,
            trim_trailing_whitespace: true,
            braces: BraceSettings::default(),
            r#if: IfSettings::default(),
            switch: SwitchSettings::default(),
            usings: UsingSettings::default(),
            ordering: OrderingRules::default(),
            files: FileOrganization::default(),
        }
    }

    #[must_use]
    pub fn from_raw(raw: &RawFormatSection) -> Self {
        let mut config = Self::default_for_env(is_ci_environment());
        if let Some(version) = raw.version {
            config.version = version;
        }
        if let Some(enabled) = raw.enabled {
            config.enabled = enabled;
        }
        if let Some(enforce) = raw.enforce.as_deref().and_then(FormatEnforcement::parse) {
            config.enforce = enforce;
        }
        if let Some(max) = raw.max_line_length {
            config.max_line_length = max.max(20);
        }
        config.indent = IndentStyle::from_raw(raw.indent.clone());
        if let Some(newline) = raw.newline.as_deref().and_then(NewlineStyle::parse) {
            config.newline = newline;
        }
        if let Some(trailing) = raw.trailing_newline {
            config.trailing_newline = trailing;
        }
        if let Some(trim) = raw.trim_trailing_whitespace {
            config.trim_trailing_whitespace = trim;
        }
        config.braces = BraceSettings::from_raw(raw.braces.clone());
        config.r#if = IfSettings::from_raw(raw.r#if.clone());
        config.switch = SwitchSettings::from_raw(raw.switch.clone());
        config.usings = UsingSettings::from_raw(raw.usings.clone());
        config.ordering = OrderingRules::from_raw(raw.ordering.clone());
        config.files = FileOrganization::from_raw(raw.files.clone());
        config
    }
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self::default_for_env(is_ci_environment())
    }
}

/// Enforcement mode for formatting violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatEnforcement {
    Off,
    Warn,
    Error,
}

impl FormatEnforcement {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" | "disable" | "none" => Some(Self::Off),
            "warn" | "warning" => Some(Self::Warn),
            "error" | "err" => Some(Self::Error),
            _ => None,
        }
    }
}

impl Default for FormatEnforcement {
    fn default() -> Self {
        Self::Warn
    }
}

/// Newline policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewlineStyle {
    Lf,
    Crlf,
}

impl NewlineStyle {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "lf" | "\n" => Some(Self::Lf),
            "crlf" | "\r\n" => Some(Self::Crlf),
            _ => None,
        }
    }
}

impl Default for NewlineStyle {
    fn default() -> Self {
        Self::Lf
    }
}

/// Indentation preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndentStyle {
    pub size: u8,
    pub use_tabs: bool,
}

impl IndentStyle {
    fn from_raw(raw: RawIndent) -> Self {
        let mut style = Self::default();
        if let Some(size) = raw.size {
            style.size = size.max(1).min(16) as u8;
        }
        if let Some(use_tabs) = raw.use_tabs {
            style.use_tabs = use_tabs;
        }
        style
    }
}

impl Default for IndentStyle {
    fn default() -> Self {
        Self {
            size: 4,
            use_tabs: false,
        }
    }
}

/// Brace placement rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BraceSettings {
    pub style: BraceStyle,
    pub require_single_line_if: bool,
    pub require_single_line_loops: bool,
}

impl BraceSettings {
    fn from_raw(raw: RawBraces) -> Self {
        let mut settings = Self::default();
        if let Some(style) = raw.style.as_deref().and_then(BraceStyle::parse) {
            settings.style = style;
        }
        if let Some(require) = raw.require_for_single_line_if {
            settings.require_single_line_if = require;
        }
        if let Some(require) = raw.require_for_single_line_loops {
            settings.require_single_line_loops = require;
        }
        settings
    }
}

impl Default for BraceSettings {
    fn default() -> Self {
        Self {
            style: BraceStyle::Allman,
            require_single_line_if: false,
            require_single_line_loops: false,
        }
    }
}

/// Supported brace styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BraceStyle {
    KAndR,
    Allman,
}

impl BraceStyle {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "k&r" | "kandr" | "knr" => Some(Self::KAndR),
            "allman" => Some(Self::Allman),
            _ => None,
        }
    }
}

/// `if`/`else` layout preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IfSettings {
    pub else_on_new_line: bool,
    pub space_before_parentheses: bool,
    pub wrap_conditions: WrapStyle,
}

impl IfSettings {
    fn from_raw(raw: RawIfSettings) -> Self {
        let mut settings = Self::default();
        if let Some(value) = raw.else_on_new_line {
            settings.else_on_new_line = value;
        }
        if let Some(value) = raw.space_before_parentheses {
            settings.space_before_parentheses = value;
        }
        if let Some(style) = raw
            .wrap_conditions
            .as_deref()
            .and_then(WrapStyle::parse_condition)
        {
            settings.wrap_conditions = style;
        }
        settings
    }
}

impl Default for IfSettings {
    fn default() -> Self {
        Self {
            else_on_new_line: true,
            space_before_parentheses: true,
            wrap_conditions: WrapStyle::IfLong,
        }
    }
}

/// Switch formatting preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchSettings {
    pub case_indent: u8,
    pub braces_style: Option<BraceStyle>,
    pub blank_line_between_cases: bool,
    pub align_case_labels: bool,
}

impl SwitchSettings {
    fn from_raw(raw: RawSwitchSettings) -> Self {
        let mut settings = Self::default();
        if let Some(indent) = raw.case_indent {
            settings.case_indent = indent.min(4);
        }
        settings.braces_style = raw.braces_style.as_deref().and_then(BraceStyle::parse);
        if let Some(blank) = raw.blank_line_between_cases {
            settings.blank_line_between_cases = blank;
        }
        if let Some(align) = raw.align_case_labels {
            settings.align_case_labels = align;
        }
        settings
    }
}

impl Default for SwitchSettings {
    fn default() -> Self {
        Self {
            case_indent: 1,
            braces_style: None,
            blank_line_between_cases: false,
            align_case_labels: true,
        }
    }
}

/// Using directive policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsingSettings {
    pub sort: bool,
    pub group: UsingGroup,
    pub blank_line_between_groups: bool,
}

impl UsingSettings {
    fn from_raw(raw: RawUsingSettings) -> Self {
        let mut settings = Self::default();
        if let Some(sort) = raw.sort {
            settings.sort = sort;
        }
        if let Some(group) = raw.group.as_deref().and_then(UsingGroup::parse) {
            settings.group = group;
        }
        if let Some(blank) = raw.blank_line_between_groups {
            settings.blank_line_between_groups = blank;
        }
        settings
    }
}

impl Default for UsingSettings {
    fn default() -> Self {
        Self {
            sort: true,
            group: UsingGroup::SystemFirst,
            blank_line_between_groups: true,
        }
    }
}

/// Using grouping policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsingGroup {
    None,
    SystemFirst,
    StdFirst,
    Custom,
}

impl UsingGroup {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" => Some(Self::None),
            "system_first" | "system-first" | "systemfirst" => Some(Self::SystemFirst),
            "std_first" | "std-first" | "stdfirst" => Some(Self::StdFirst),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

/// Ordering preferences for types and members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderingRules {
    pub types: Vec<TypeSort>,
    pub members: Vec<MemberSort>,
    pub access: Vec<AccessSort>,
}

impl OrderingRules {
    fn from_raw(raw: RawOrderingRules) -> Self {
        let mut rules = Self::default();
        if !raw.types.is_empty() {
            rules.types = raw
                .types
                .iter()
                .filter_map(|entry| TypeSort::parse(entry))
                .collect();
        }
        if !raw.members.is_empty() {
            rules.members = raw
                .members
                .iter()
                .filter_map(|entry| MemberSort::parse(entry))
                .collect();
        }
        if !raw.access.is_empty() {
            let parsed: Vec<AccessSort> = raw
                .access
                .iter()
                .filter_map(|entry| AccessSort::parse(entry))
                .collect();
            if !parsed.is_empty() {
                rules.access = parsed;
            }
        }
        rules
    }
}

impl Default for OrderingRules {
    fn default() -> Self {
        Self {
            types: Vec::new(),
            members: Vec::new(),
            access: vec![
                AccessSort::Public,
                AccessSort::Internal,
                AccessSort::Protected,
                AccessSort::Private,
            ],
        }
    }
}

/// Preferred ordering of type declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeSort {
    Struct,
    Enum,
    Class,
    Interface,
    Trait,
    Extension,
    Impl,
    Delegate,
}

impl TypeSort {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "struct" => Some(Self::Struct),
            "enum" => Some(Self::Enum),
            "class" => Some(Self::Class),
            "interface" => Some(Self::Interface),
            "trait" => Some(Self::Trait),
            "extension" => Some(Self::Extension),
            "impl" => Some(Self::Impl),
            "delegate" => Some(Self::Delegate),
            _ => None,
        }
    }
}

/// Preferred ordering of members inside a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberSort {
    Fields,
    Constructors,
    Properties,
    Methods,
    NestedTypes,
    Operators,
    Consts,
    Statics,
}

impl MemberSort {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "field" | "fields" => Some(Self::Fields),
            "ctor" | "ctors" | "constructor" | "constructors" => Some(Self::Constructors),
            "prop" | "props" | "property" | "properties" => Some(Self::Properties),
            "method" | "methods" => Some(Self::Methods),
            "nested" | "types" | "nested_types" | "nested-types" => Some(Self::NestedTypes),
            "operator" | "operators" => Some(Self::Operators),
            "const" | "consts" | "constants" => Some(Self::Consts),
            "static" | "statics" => Some(Self::Statics),
            _ => None,
        }
    }
}

/// Access modifier ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessSort {
    Public,
    Internal,
    Protected,
    Private,
}

impl AccessSort {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "public" => Some(Self::Public),
            "internal" => Some(Self::Internal),
            "protected" => Some(Self::Protected),
            "private" => Some(Self::Private),
            _ => None,
        }
    }
}

/// File organization assistance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOrganization {
    pub one_top_level_type_per_file: bool,
    pub require_filename_match: bool,
    pub action: FileOrganizationAction,
    pub naming: FileNaming,
}

impl FileOrganization {
    fn from_raw(raw: RawFileOrganization) -> Self {
        let mut org = Self::default();
        if let Some(value) = raw.one_top_level_type_per_file {
            org.one_top_level_type_per_file = value;
        }
        if let Some(value) = raw.require_filename_match {
            org.require_filename_match = value;
        }
        if let Some(action) = raw
            .action
            .as_deref()
            .and_then(FileOrganizationAction::parse)
        {
            org.action = action;
        }
        if let Some(naming) = raw.naming.as_deref().and_then(FileNaming::parse) {
            org.naming = naming;
        }
        org
    }
}

impl Default for FileOrganization {
    fn default() -> Self {
        Self {
            one_top_level_type_per_file: true,
            require_filename_match: true,
            action: FileOrganizationAction::Suggest,
            naming: FileNaming::TypeName,
        }
    }
}

/// Whether file-organization suggestions should be applied automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOrganizationAction {
    Suggest,
    Apply,
}

impl FileOrganizationAction {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "suggest" => Some(Self::Suggest),
            "apply" => Some(Self::Apply),
            _ => None,
        }
    }
}

/// File naming policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileNaming {
    TypeName,
    NamespaceQualified,
}

impl FileNaming {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "typename" | "type_name" | "type-name" => Some(Self::TypeName),
            "namespace+typename" | "namespace+type" | "namespace_type" | "namespace-type" => {
                Some(Self::NamespaceQualified)
            }
            _ => None,
        }
    }
}

/// Break/wrap style for multi-part constructs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapStyle {
    Never,
    IfLong,
    Always,
}

impl WrapStyle {
    #[must_use]
    pub fn parse_condition(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "never" => Some(Self::Never),
            "if_long" | "if-long" | "iflong" => Some(Self::IfLong),
            "always" => Some(Self::Always),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RawFormatSection {
    #[serde(default)]
    pub version: Option<u32>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub enforce: Option<String>,
    #[serde(default)]
    pub max_line_length: Option<usize>,
    #[serde(default)]
    pub indent: RawIndent,
    #[serde(default)]
    pub newline: Option<String>,
    #[serde(default)]
    pub trailing_newline: Option<bool>,
    #[serde(default)]
    pub trim_trailing_whitespace: Option<bool>,
    #[serde(default)]
    pub braces: RawBraces,
    #[serde(default, rename = "if")]
    pub r#if: RawIfSettings,
    #[serde(default)]
    pub switch: RawSwitchSettings,
    #[serde(default)]
    pub usings: RawUsingSettings,
    #[serde(default)]
    pub ordering: RawOrderingRules,
    #[serde(default)]
    pub files: RawFileOrganization,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RawIndent {
    #[serde(default)]
    pub size: Option<u32>,
    #[serde(default, rename = "use_tabs")]
    pub use_tabs: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RawBraces {
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub require_for_single_line_if: Option<bool>,
    #[serde(default)]
    pub require_for_single_line_loops: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RawIfSettings {
    #[serde(default)]
    pub else_on_new_line: Option<bool>,
    #[serde(default)]
    pub space_before_parentheses: Option<bool>,
    #[serde(default)]
    pub wrap_conditions: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RawSwitchSettings {
    #[serde(default)]
    pub case_indent: Option<u8>,
    #[serde(default)]
    pub braces_style: Option<String>,
    #[serde(default)]
    pub blank_line_between_cases: Option<bool>,
    #[serde(default)]
    pub align_case_labels: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RawUsingSettings {
    #[serde(default)]
    pub sort: Option<bool>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub blank_line_between_groups: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RawOrderingRules {
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default)]
    pub access: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RawFileOrganization {
    #[serde(default)]
    pub one_top_level_type_per_file: Option<bool>,
    #[serde(default)]
    pub require_filename_match: Option<bool>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub naming: Option<String>,
}

fn is_ci_environment() -> bool {
    env::var("CHIC_CI")
        .or_else(|_| env::var("CI"))
        .map(|value| {
            let lower = value.trim().to_ascii_lowercase();
            !(lower.is_empty() || matches!(lower.as_str(), "0" | "false" | "off" | "no"))
        })
        .unwrap_or(false)
}
