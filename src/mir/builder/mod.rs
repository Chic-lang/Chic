//! Lower Chic AST into MIR skeletons.

pub(super) use crate::frontend::ast::{
    BindingModifier, Block, EnumDecl, Expression, ExtensionDecl, ExtensionMember, FieldDecl,
    GotoStatement, GotoTarget, InterfaceDecl, InterfaceMember, Item, Module, Parameter, Signature,
    Statement as AstStatement, StatementKind as AstStatementKind, StructDecl, SwitchLabel,
    SwitchStatement, TryStatement, UnionDecl, UnionMember, VariableDeclaration, VariableModifier,
    Visibility,
};
pub(super) use crate::frontend::diagnostics::Span;
pub(super) use crate::frontend::lexer::{Keyword, Token, TokenKind, lex};
pub(super) use crate::syntax::pattern::{
    BindingPatternNode, ListPatternNode, PatternAst, PatternBinaryOp, PatternNode, RelationalOp,
    VariantPatternFieldsNode, parse_pattern,
};
pub(super) use crate::typeck::{AutoTraitKind, ConstraintKind, TypeConstraint};

pub(super) use crate::mir::data::{
    Abi, BasicBlock, BinOp, BindingPattern, BlockId, BorrowId, BorrowKind, BorrowOperand,
    ConstValue, FnSig, FnTy, FunctionKind, LocalDecl, LocalId, LocalKind, MatchArm, MatchGuard,
    MirBody, MirFunction, MirModule, MmioOperand, Mutability, Operand, ParamMode, Pattern,
    PatternBinding, PatternBindingMode, PatternBindingMutability, PatternField,
    PatternProjectionElem, PendingOperand, PendingRvalue, PendingStatement, PendingStatementKind,
    Place, ProjectionElem, RegionVar, Rvalue, Statement as MirStatement,
    StatementKind as MirStatementKind, StrLifetime, Terminator, TraitObjectDispatch, TraitVTable,
    Ty, UnOp, ValueCategory, VariantPatternFields,
};
pub(super) use crate::mir::layout::{
    AutoTraitOverride, AutoTraitSet, ClassLayoutKind, EnumLayout, EnumVariantLayout, FieldLayout,
    ListLayout, MmioAccess, PositionalElement, StructLayout, TypeLayout, TypeLayoutTable, TypeRepr,
    UnionFieldLayout, UnionFieldMode, UnionLayout,
};
pub(super) use crate::mir::state::{
    AsyncStateMachine, AsyncSuspendPoint, CatchFilter, CatchRegion, ExceptionRegion, FinallyRegion,
    GeneratorStateMachine, GeneratorYieldPoint,
};

pub(super) use super::expr::{AssignOp, LambdaBody, LambdaParamModifier, parse_expression};
#[allow(unused_imports)]
pub(super) use crate::syntax::expr::ExprNode;
pub mod accelerator;
mod body_builder;
mod const_eval;
mod constructors;
mod default_arguments;
mod functions;
mod module_lowering;
mod specialization;
mod static_registry;
mod string_interner;
mod support;
pub(crate) mod symbol_index;
use body_builder::BodyBuilder;
pub(crate) use body_builder::drop_lowering::synthesise_drop_statements;
pub use const_eval::{ConstEvalContext, ConstEvalSummary};
pub(crate) use default_arguments::{DefaultArgumentStore, DefaultArgumentValue};
pub use module_lowering::driver::PassStageMetric;
pub use module_lowering::{
    LoweringDiagnostic, LoweringResult, ModuleUnitSlice, lower_module, lower_module_with_units,
    lower_module_with_units_and_hook,
};
pub(crate) use specialization::{FunctionSpecialization, specialised_function_name};
pub(super) use string_interner::StringInterner;
use support::{
    BindingSpec, CasePatternKind, GuardMetadata, LabelState, ParsedCasePattern, PendingGoto,
    ScopeFrame, ScopeLocal, ScopeLocalSnapshot, ScopeSnapshot, SwitchCase, SwitchContext,
    SwitchSectionInfo, SwitchTarget, TryContext, is_pin_type_name, literal_key_from_const,
    switch_section_span,
};
pub(crate) use symbol_index::FunctionSymbol;
pub use symbol_index::{FieldMetadata, PropertyMetadata, SymbolIndex};

fn qualify(namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(prefix) if !prefix.is_empty() => {
            let mut prefix_parts: Vec<String> = prefix
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();
            let name_parts: Vec<String> = name
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();

            if !prefix_parts.is_empty()
                && name_parts.len() >= prefix_parts.len()
                && name_parts[..prefix_parts.len()] == prefix_parts[..]
            {
                name_parts.join("::")
            } else if name_parts.is_empty() {
                prefix_parts.join("::")
            } else {
                prefix_parts.extend(name_parts);
                prefix_parts.join("::")
            }
        }
        _ => name.to_string(),
    }
}

pub(crate) fn pointer_size() -> usize {
    crate::mir::layout::pointer_size()
}

pub(crate) fn pointer_align() -> usize {
    crate::mir::layout::pointer_align()
}

pub(crate) const MIN_ALIGN: usize = 1;

fn align_to(value: usize, align: usize) -> usize {
    if align <= 1 {
        value
    } else {
        value.div_ceil(align) * align
    }
}

#[cfg(test)]
mod tests;
