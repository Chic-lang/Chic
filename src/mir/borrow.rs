//! Borrow checking over Chic MIR.

use crate::frontend::diagnostics::{Diagnostic, Severity};

use super::data::{Abi, MirFunction, MirModule};
use super::layout::TypeLayoutTable;

mod context;
use context::BorrowChecker;

/// Result of borrow checking a MIR entity.
#[derive(Debug, Default)]
pub struct BorrowCheckResult {
    pub diagnostics: Vec<Diagnostic>,
}

impl BorrowCheckResult {
    pub fn merge(&mut self, mut other: BorrowCheckResult) {
        self.diagnostics.append(&mut other.diagnostics);
    }

    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diag| diag.severity != Severity::Error)
    }
}

/// Borrow check an entire MIR module.
#[must_use]
pub fn borrow_check_module(module: &MirModule) -> BorrowCheckResult {
    let mut result = BorrowCheckResult::default();
    for function in &module.functions {
        result.merge(borrow_check_function_with_layouts(
            function,
            &module.type_layouts,
        ));
    }
    result
}

/// Borrow check a single MIR function.
#[must_use]
pub fn borrow_check_function(function: &MirFunction) -> BorrowCheckResult {
    let layouts = TypeLayoutTable::default();
    borrow_check_function_with_layouts(function, &layouts)
}

#[must_use]
pub fn borrow_check_function_with_layouts(
    function: &MirFunction,
    layouts: &TypeLayoutTable,
) -> BorrowCheckResult {
    if matches!(function.signature.abi, Abi::Extern(_)) || function.body.blocks.is_empty() {
        return BorrowCheckResult::default();
    }
    let mut checker = BorrowChecker::new(function, layouts);
    checker.run()
}

#[cfg(test)]
mod tests;
