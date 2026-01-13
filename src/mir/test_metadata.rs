use blake3::hash;

use crate::frontend::diagnostics::Span;
use crate::mir::{FunctionKind, MirModule};

/// Metadata describing a testcase discovered during MIR lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCaseMetadata {
    /// Index of the testcase function inside `MirModule::functions`.
    pub function_index: usize,
    /// Stable identifier derived from the qualified name (or an explicit `@id` override).
    pub id: String,
    /// Fully-qualified function name recorded in MIR (after internal ordinal decoration).
    pub qualified_name: String,
    /// Short display name (last segment of the qualified name).
    pub name: String,
    /// Namespace containing the testcase, if present.
    pub namespace: Option<String>,
    /// User-provided categories/tags (lowercased, de-duplicated).
    pub categories: Vec<String>,
    /// Parameter metadata for data-driven scenarios.
    pub parameters: Vec<TestCaseParameterMetadata>,
    /// Whether the testcase is async.
    pub is_async: bool,
    /// Source span for the declaration.
    pub span: Option<Span>,
}

/// Parameter metadata captured for testcase declarations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCaseParameterMetadata {
    /// Parameter name as declared.
    pub name: String,
    /// Surface type string for the parameter, if known.
    pub ty: Option<String>,
    /// Whether the parameter has a default value.
    pub has_default: bool,
}

impl TestCaseMetadata {
    #[must_use]
    pub fn stable_id(qualified_name: &str, override_id: Option<&str>) -> String {
        if let Some(id) = override_id {
            return id.to_string();
        }
        let digest = hash(qualified_name.as_bytes());
        format!("t-{}", &digest.to_hex()[..12])
    }

    #[must_use]
    pub fn split_namespace(name: &str) -> (Option<String>, String) {
        let mut parts = name.rsplitn(2, "::");
        let leaf = parts.next().unwrap_or(name).to_string();
        let namespace = parts.next().map(str::to_string);
        (namespace, leaf)
    }
}

/// Collect testcase metadata from a MIR module, falling back to function inspection
/// when explicit metadata is absent.
#[must_use]
pub fn collect_test_metadata(module: &MirModule) -> Vec<TestCaseMetadata> {
    if !module.test_cases.is_empty() {
        let mut repaired = Vec::with_capacity(module.test_cases.len());
        for meta in &module.test_cases {
            let mut updated = meta.clone();
            let matches_index = module
                .functions
                .get(updated.function_index)
                .is_some_and(|func| func.name == updated.qualified_name);
            if !matches_index {
                if let Some((idx, func)) = module
                    .functions
                    .iter()
                    .enumerate()
                    .find(|(_, func)| func.name == updated.qualified_name)
                {
                    updated.function_index = idx;
                    updated.is_async = func.is_async;
                } else {
                    continue;
                }
            }
            if module
                .functions
                .get(updated.function_index)
                .is_some_and(|func| matches!(func.kind, FunctionKind::Testcase))
            {
                repaired.push(updated);
            }
        }
        return repaired;
    }
    module
        .functions
        .iter()
        .enumerate()
        .filter_map(|(index, function)| {
            if !matches!(function.kind, FunctionKind::Testcase) {
                return None;
            }
            let (namespace, name) = TestCaseMetadata::split_namespace(&function.name);
            let id = TestCaseMetadata::stable_id(&function.name, None);
            Some(TestCaseMetadata {
                function_index: index,
                id,
                qualified_name: function.name.clone(),
                name,
                namespace,
                categories: Vec::new(),
                parameters: Vec::new(),
                is_async: function.is_async,
                span: function.span,
            })
        })
        .collect()
}
