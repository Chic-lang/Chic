use super::basic_blocks::MirBody;
use super::types::{Abi, Ty};
use crate::frontend::ast::{ExternBinding, ExternOptions};
use crate::frontend::attributes::OptimizationHints;
use crate::frontend::diagnostics::Span;

/// MIR representation for an individual function or testcase.
#[derive(Debug, Clone)]
pub struct MirFunction {
    pub name: String,
    pub kind: FunctionKind,
    pub signature: FnSig,
    pub body: MirBody,
    pub is_async: bool,
    pub async_result: Option<Ty>,
    pub is_generator: bool,
    pub span: Option<Span>,
    pub optimization_hints: OptimizationHints,
    pub extern_spec: Option<MirExternSpec>,
    pub is_weak: bool,
    pub is_weak_import: bool,
}

impl MirFunction {
    #[must_use]
    pub fn is_local(&self) -> bool {
        self.name.contains("::local$")
            || (!self.name.contains("::") && self.name.starts_with("local$"))
    }
}

/// Identifies the flavour of function represented by the MIR body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionKind {
    Function,
    Testcase,
    Method,
    Constructor,
}

/// Function signature lowered alongside MIR.
#[derive(Debug, Clone)]
pub struct FnSig {
    pub params: Vec<Ty>,
    pub ret: Ty,
    pub abi: Abi,
    pub effects: Vec<Ty>,
    /// Borrow/lending metadata on the return value, listing parameter names that it may borrow from.
    pub lends_to_return: Option<Vec<String>>,
    pub variadic: bool,
}

/// Metadata describing how an extern function should be resolved/emitted.
#[derive(Debug, Clone)]
pub struct MirExternSpec {
    pub convention: String,
    pub library: Option<String>,
    pub alias: Option<String>,
    pub binding: ExternBinding,
    pub optional: bool,
    pub charset: Option<String>,
    pub weak: bool,
}

impl MirExternSpec {
    #[must_use]
    pub fn from_ast(options: Option<&ExternOptions>, fallback_abi: &str) -> Self {
        let convention = options
            .map(|opts| opts.convention.clone())
            .unwrap_or_else(|| fallback_abi.to_string());
        let binding = options
            .map(|opts| opts.binding)
            .unwrap_or(ExternBinding::Static);
        let optional = options.map(|opts| opts.optional).unwrap_or(false);
        Self {
            convention,
            library: options.and_then(|opts| opts.library.clone()),
            alias: options.and_then(|opts| opts.alias.clone()),
            binding,
            optional,
            charset: options.and_then(|opts| opts.charset.clone()),
            weak: false,
        }
    }
}

impl FnSig {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            params: Vec::new(),
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),
            lends_to_return: None,
            variadic: false,
        }
    }
}
