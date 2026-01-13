//! Lint engine: configuration, diagnostics, and MIR/HIR-integrated passes.

mod allow;
mod config;
mod diagnostic;
mod passes;

pub use config::{LintConfig, discover, discover_with_override};
pub use diagnostic::{LintCategory, LintDescriptor, LintDiagnostic, LintLevel, LintSuggestion};
pub use passes::run_lints;

#[derive(Debug, Clone, Copy)]
pub struct LintModuleInfo<'a> {
    pub path: &'a std::path::Path,
    pub is_stdlib: bool,
}

pub(crate) fn descriptors() -> &'static [LintDescriptor] {
    &DESCRIPTORS
}

pub(crate) fn find_descriptor(name: &str) -> Option<&'static LintDescriptor> {
    let canonical = canonical_lint_name(name);
    DESCRIPTORS.iter().find(|descriptor| {
        descriptor.name == canonical || descriptor.code.eq_ignore_ascii_case(name)
    })
}

pub(crate) fn canonical_lint_name(raw: &str) -> String {
    raw.trim_matches(|c: char| c == '"' || c == '\'')
        .trim()
        .replace('-', "_")
        .to_ascii_lowercase()
}

static DESCRIPTORS: &[LintDescriptor] = &[
    LintDescriptor {
        code: "LINT001",
        name: "dead_code",
        description: "detects declarations that are never referenced",
        category: LintCategory::Correctness,
        default_level: LintLevel::Warn,
    },
    LintDescriptor {
        code: "LINT002",
        name: "unused_param",
        description: "parameters that are declared but never read",
        category: LintCategory::Style,
        default_level: LintLevel::Warn,
    },
    LintDescriptor {
        code: "LINT003",
        name: "type_named_constructor",
        description: "constructors must be declared with `init(...)` instead of repeating the type name",
        category: LintCategory::Correctness,
        default_level: LintLevel::Error,
    },
];
