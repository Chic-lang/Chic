#![cfg(test)]
#![allow(unused_imports)]

use super::common::*;
use super::helpers::*;
use super::*;
use crate::chic_kind::ChicKind;
use crate::codegen::wasm::emitter::function::{LocalRepresentation, VecIndexAccess, VecIndexKind};
use crate::codegen::wasm::{RuntimeHook, push_i32_const, push_i64_const};
use crate::drop_glue::drop_glue_symbol_for;
use crate::frontend::ast::{
    Block as AstBlock, FunctionDecl, Item as AstItem, Module as AstModule, NamespaceDecl,
    Signature, TypeExpr, Visibility,
};
use crate::mir::{
    Abi, AutoTraitOverride, AutoTraitSet, BasicBlock, BinOp, BlockId, BorrowKind, BorrowOperand,
    CastKind, ClassLayoutInfo, ClassLayoutKind, ConstValue, EnumLayout, EnumVariantLayout,
    FieldLayout, FnSig, FnTy, FunctionKind, LocalDecl, LocalId, LocalKind, MatchArm, MirBody,
    MirFunction, MirModule, MmioEndianness, Operand, Pattern, PatternBinding, PendingOperand,
    PendingRvalue, PendingTerminator, PendingTerminatorKind, Place, ProjectionElem, RegionVar,
    Rvalue, Statement, StatementKind, StructLayout, Terminator, Ty, TypeLayout, TypeLayoutTable,
    TypeRepr, UnOp, UnionFieldLayout, UnionFieldMode, UnionLayout, ValueCategory,
};
use crate::mmio::{AddressSpaceId, encode_flags};
use crate::runtime::error::exception_type_identity;
use crate::runtime::wasm_executor::hooks::PANIC_EXIT_CODE;
use crate::target::Target;
use std::collections::HashMap;

#[test]
fn emit_body_lowers_yield_terminators() {
    let function = function_with_yield();
    let body = emit_body_using_layouts(wasm_layouts(), function, |_| None);
    assert!(
        body.iter().any(|byte| *byte == 0x10),
        "lowered body should invoke runtime yield hook"
    );
}
#[test]
fn emit_body_lowers_await_terminators() {
    let function = async_single_await_function();
    let body = emit_body_using_layouts(wasm_layouts(), function, |_| None);
    assert!(
        body.iter().any(|byte| *byte == 0x10),
        "lowered body should invoke runtime await hook"
    );
}
#[test]
fn emit_body_rejects_non_i32_future_handles() {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("future".into()),
        Ty::named("double"),
        true,
        None,
        LocalKind::Local,
    ));
    body.locals.push(LocalDecl::new(
        Some("result".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Await {
        future: Place::new(LocalId(1)),
        destination: Some(Place::new(LocalId(2))),
        resume: BlockId(1),
        drop: BlockId(2),
    });
    body.blocks.push(entry);
    body.blocks.push(simple_return_block(1));
    body.blocks.push(simple_return_block(2));

    let function = MirFunction {
        name: "Demo::BadAwait".into(),
        kind: FunctionKind::Function,
        signature: FnSig::empty(),
        body,
        is_async: true,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    };

    with_emitter_using_layouts(
        wasm_layouts(),
        function,
        |_| None,
        |emitter| {
            let err = emitter
                .emit_body()
                .expect_err("await lowering should reject non-i32 futures");
            assert!(
                format!("{err}").contains("i32 future handles"),
                "unexpected error message: {err}"
            );
        },
    );
}
