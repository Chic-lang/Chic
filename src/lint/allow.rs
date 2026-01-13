use std::collections::HashSet;

use crate::frontend::ast::Attribute;

use super::diagnostic::LintCategory;
use super::{canonical_lint_name, descriptors};

#[derive(Debug, Clone, Default)]
pub struct LintAllowance {
    allow_all: bool,
    allowed_rules: HashSet<String>,
    allowed_categories: HashSet<LintCategory>,
}

impl LintAllowance {
    #[must_use]
    pub fn allows(&self, category: LintCategory, rule: &str) -> bool {
        if self.allow_all {
            return true;
        }
        let canonical = canonical_lint_name(rule);
        self.allowed_rules.contains(&canonical) || self.allowed_categories.contains(&category)
    }

    #[must_use]
    pub fn merged(&self, other: &Self) -> Self {
        let mut merged = self.clone();
        if other.allow_all {
            merged.allow_all = true;
        }
        merged
            .allowed_rules
            .extend(other.allowed_rules.iter().cloned());
        merged
            .allowed_categories
            .extend(other.allowed_categories.iter().cloned());
        merged
    }
}

pub(crate) fn allowance_from_attributes(attrs: &[Attribute]) -> LintAllowance {
    let mut allowance = LintAllowance::default();

    for attr in attrs {
        let lowered = canonical_lint_name(&attr.name);
        if lowered == "allow" {
            for argument in &attr.arguments {
                let parts = argument.value.split(',');
                for value in parts {
                    let canonical = canonical_lint_name(value);
                    if canonical == "all" {
                        allowance.allow_all = true;
                        continue;
                    }
                    if let Some(category) = LintCategory::from_str(&canonical) {
                        allowance.allowed_categories.insert(category);
                    } else {
                        allowance.allowed_rules.insert(canonical);
                    }
                }
            }
            continue;
        }
        if let Some(category) = LintCategory::from_str(&lowered) {
            allowance.allowed_categories.insert(category);
            continue;
        }
        if descriptors().iter().any(|descriptor| {
            descriptor.name == lowered || descriptor.code.eq_ignore_ascii_case(&attr.name)
        }) {
            allowance.allowed_rules.insert(lowered.clone());
        }
    }

    allowance
}
