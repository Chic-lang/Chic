#![cfg(test)]
#![allow(unused_imports)]

use super::common::*;
use super::helpers::*;
use super::*;
use crate::chic_kind::ChicKind;
use crate::codegen::wasm::emitter::function::{LocalRepresentation, VecIndexAccess, VecIndexKind};
use crate::codegen::wasm::module_builder::FunctionSignature;
use crate::codegen::wasm::{RuntimeHook, push_i32_const, push_i64_const};
use crate::drop_glue::drop_type_identity;
use crate::frontend::ast::{
    Block as AstBlock, FunctionDecl, Item as AstItem, Module as AstModule, NamespaceDecl,
    Signature, TypeExpr, Visibility,
};
use crate::mir::{
    Abi, AggregateKind, AutoTraitOverride, AutoTraitSet, BasicBlock, BinOp, BlockId, BorrowKind,
    BorrowOperand, CastKind, ClassLayoutInfo, ClassLayoutKind, ConstOperand, ConstValue,
    EnumLayout, EnumVariantLayout, FieldLayout, FnSig, FnTy, FunctionKind, LocalDecl, LocalId,
    LocalKind, MatchArm, MirBody, MirFunction, MirModule, MmioEndianness, Operand, Pattern,
    PatternBinding, PendingOperand, PendingRvalue, PendingTerminator, PendingTerminatorKind, Place,
    ProjectionElem, RegionVar, Rvalue, Statement, StatementKind, StructLayout, Terminator, Ty,
    TypeLayout, TypeLayoutTable, TypeRepr, UnOp, UnionFieldLayout, UnionFieldMode, UnionLayout,
    ValueCategory,
};
use crate::mmio::{AddressSpaceId, encode_flags};
use crate::runtime::error::exception_type_identity;
use crate::runtime::wasm_executor::hooks::PANIC_EXIT_CODE;
use crate::target::Target;
use std::collections::HashMap;

#[test]
fn emit_call_indirect_handles_function_pointer_operands() {
    let fn_ty = FnTy::new(Vec::new(), Ty::Unit, Abi::Chic);
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("ptr".into()),
        Ty::Fn(fn_ty.clone()),
        false,
        None,
        LocalKind::Temp,
    ));

    let assign_pointer = Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Aggregate {
                kind: AggregateKind::Adt {
                    name: fn_ty.canonical_name(),
                    variant: None,
                },
                fields: vec![
                    Operand::Const(ConstOperand::new(ConstValue::Symbol("Demo::Target".into()))),
                    Operand::Const(ConstOperand::new(ConstValue::Null)),
                    Operand::Const(ConstOperand::new(ConstValue::Null)),
                    Operand::Const(ConstOperand::new(ConstValue::UInt(
                        drop_type_identity(&fn_ty.canonical_name()).into(),
                    ))),
                    Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
                    Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
                ],
            },
        },
    };

    let call_terminator = Terminator::Call {
        func: Operand::Copy(Place::new(LocalId(1))),
        args: Vec::new(),
        arg_modes: Vec::new(),
        destination: None,
        target: BlockId(1),
        unwind: None,

        dispatch: None,
    };

    let block0 = BasicBlock {
        id: BlockId(0),
        statements: vec![assign_pointer],
        terminator: Some(call_terminator),
        span: None,
    };
    let block1 = BasicBlock {
        id: BlockId(1),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    };
    body.blocks.push(block0);
    body.blocks.push(block1);

    let function = MirFunction {
        name: "Demo::Entry".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        },
        body,
        is_async: false,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    };

    let mut layouts = wasm_layouts();
    layouts.ensure_fn_layout(&fn_ty);

    let mut signatures = HashMap::new();
    signatures.insert(FunctionSignature::from_fn_ty(&fn_ty, &layouts), 3);

    let body_bytes = emit_body_using_layouts(layouts, function, move |indices| {
        indices.insert("Demo::Target".into(), 5);
        Some(signatures)
    });

    assert!(
        body_contains_call_indirect(&body_bytes, 3, 0),
        "expected call_indirect in emitted body"
    );
    assert!(
        body_bytes.contains(&0x45),
        "function pointer guard should include i32.eqz"
    );
    assert!(
        body_bytes.contains(&0x22),
        "function pointer guard should store operand via local.tee"
    );
}
#[test]
fn emit_body_resolves_pending_call_targets() {
    let operand = Operand::Pending(PendingOperand {
        category: ValueCategory::Pending,
        repr: "Demo.Callee".into(),
        span: None,
        info: None,
    });
    let function = call_function_with_operand(operand);
    let body = emit_body_using_layouts(wasm_layouts(), function, |indices| {
        indices.insert("Demo::Callee".into(), 3);
        None
    });
    assert!(
        body.windows(1).any(|window| window[0] == 0x10),
        "generated body should contain a call opcode"
    );
}
#[test]
fn emit_body_resolves_string_call_targets() {
    let function = call_function_with_string_operand("Demo::Callee");
    let body = emit_body_using_layouts(wasm_layouts(), function, |indices| {
        indices.insert("Demo::Callee".into(), 5);
        None
    });
    assert!(
        body.windows(1).any(|window| window[0] == 0x10),
        "generated body should contain a call opcode"
    );
}
#[test]
fn emit_body_rejects_unknown_call_target() {
    let operand = Operand::Pending(PendingOperand {
        category: ValueCategory::Pending,
        repr: "Demo.Callee".into(),
        span: None,
        info: None,
    });
    let function = call_function_with_operand(operand);
    with_emitter_using_layouts(
        wasm_layouts(),
        function,
        |_| None,
        |emitter| {
            let err = emitter
                .emit_body()
                .expect_err("missing call target should fail lowering");
            assert!(
                format!("{err}").contains("unable to resolve call target"),
                "unexpected error message: {err}"
            );
        },
    );
}
#[test]
fn emit_body_rejects_first_class_function_operands() {
    let operand = Operand::Copy(Place::new(LocalId(1)));
    let function = call_function_with_operand(operand);
    with_emitter_using_layouts(
        wasm_layouts(),
        function,
        |indices| {
            indices.insert("Demo::Callee".into(), 3);
            None
        },
        |emitter| {
            let err = emitter
                .emit_body()
                .expect_err("first-class function operands are not supported");
            assert!(
                format!("{err}").contains("first-class function values"),
                "unexpected error message: {err}"
            );
        },
    );
}
#[test]
fn emit_body_handles_float_constants() {
    let function = float_constant_function();
    let body = emit_body_using_layouts(wasm_layouts(), function, |_| None);
    assert!(
        body.windows(1).any(|window| window[0] == 0x44),
        "expected f64.const opcode (0x44) in function body: {body:?}"
    );
    assert!(
        body.windows(1).any(|window| window[0] == 0xB6),
        "expected f32.demote_f64 opcode (0xB6) converting to local type: {body:?}"
    );
}
#[test]
fn emit_operand_uses_zero_for_non_addressable_local() {
    let function = function_with_return_arg_call();
    let body = emit_body_using_layouts(wasm_layouts(), function, |indices| {
        indices.insert("Demo::Callee".into(), 3);
        None
    });
    assert!(
        body.windows(2).any(|w| w == [0x41, 0x00]),
        "non-addressable locals should fall back to pushing zero"
    );
}
#[test]
fn emit_call_stores_result_into_destination_local() {
    let function = function_with_call_destination();
    let body = emit_body_using_layouts(wasm_layouts(), function, |indices| {
        indices.insert("Demo::Callee".into(), 7);
        None
    });
    assert!(
        body.windows(1).any(|window| window[0] == 0x21),
        "expected call lowering to emit local.set for destination"
    );
}
