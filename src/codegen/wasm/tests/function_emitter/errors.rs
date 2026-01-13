#![cfg(test)]
#![allow(unused_imports)]

use super::common::*;
use super::helpers::*;
use super::*;
use crate::codegen::wasm::{RuntimeHook, push_i32_const};
use crate::mir::{
    BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, InlineAsm, InlineAsmOperand,
    InlineAsmOperandKind, InlineAsmOptions, InlineAsmRegister, InlineAsmRegisterClass,
    InlineAsmTemplatePiece, LocalDecl, LocalId, LocalKind, MirBody, MirFunction, Operand, Place,
    Rvalue, Statement, StatementKind, Terminator, Ty, TypeLayoutTable,
};
use crate::runtime::wasm_executor::hooks::PANIC_EXIT_CODE;

#[test]
fn emit_terminator_panic_emits_trap() {
    let function = function_with_panic();
    let (harness, func_name) = harness_with_layouts(function, wasm_layouts());
    let hooks = harness.runtime_hooks().clone();
    let body = harness
        .emit_body_with(&func_name, |_| None)
        .unwrap_or_else(|err| panic!("emit panic body: {err}"));
    let panic_index = hooks
        .get(&RuntimeHook::Panic.qualified_name())
        .copied()
        .expect("panic hook index present");
    assert!(
        body_contains_call(&body, panic_index),
        "panic lowering should call the runtime panic hook"
    );
    let mut expected_const = Vec::new();
    push_i32_const(&mut expected_const, PANIC_EXIT_CODE);
    assert!(
        body.windows(expected_const.len())
            .any(|bytes| bytes == expected_const.as_slice()),
        "panic lowering should push the deterministic exit code"
    );
    assert!(
        body.contains(&0x00),
        "panic lowering should remain unreachable afterward"
    );
}

#[test]
fn emit_terminator_throw_invokes_runtime_hook() {
    let function = function_with_throw();
    let (harness, func_name) = harness_with_layouts(function, wasm_layouts());
    let hooks = harness.runtime_hooks().clone();
    let body = harness
        .emit_body_with(&func_name, |_| None)
        .unwrap_or_else(|err| panic!("emit throw body: {err}"));
    let throw_index = hooks
        .get(&RuntimeHook::Throw.qualified_name())
        .copied()
        .expect("throw hook index present");
    assert!(
        body_contains_call(&body, throw_index),
        "throw lowering should call the runtime throw hook"
    );
}

#[test]
fn inline_asm_is_rejected_by_wasm_backend() {
    let function = inline_asm_function();
    let (harness, func_name) = harness_with_layouts(function, wasm_layouts());
    let err = harness
        .emit_body_with(&func_name, |_| None)
        .expect_err("inline asm should not lower for wasm");
    assert!(
        format!("{err:?}").contains("inline assembly"),
        "unexpected error: {err}"
    );
}

fn inline_asm_function() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("dst".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Local,
    ));
    let mut block = crate::mir::BasicBlock::new(crate::mir::BlockId(0), None);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(1)))),
        },
    });
    let asm = InlineAsm {
        template: vec![
            InlineAsmTemplatePiece::Literal("xor ".into()),
            InlineAsmTemplatePiece::Placeholder {
                operand_idx: 0,
                modifier: None,
                span: None,
            },
            InlineAsmTemplatePiece::Literal(", ".into()),
            InlineAsmTemplatePiece::Placeholder {
                operand_idx: 0,
                modifier: None,
                span: None,
            },
        ],
        operands: vec![InlineAsmOperand {
            name: Some("dst".into()),
            reg: InlineAsmRegister::Class(InlineAsmRegisterClass::Reg),
            kind: InlineAsmOperandKind::InOut {
                input: Operand::Copy(Place::new(LocalId(1))),
                output: Place::new(LocalId(1)),
                late: true,
            },
            span: None,
        }],
        clobbers: Vec::new(),
        options: InlineAsmOptions {
            volatile: true,
            ..InlineAsmOptions::default()
        },
        span: None,
    };
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::InlineAsm(asm),
    });
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    MirFunction {
        name: "Sample::InlineAsm".into(),
        kind: crate::mir::FunctionKind::Function,
        signature: FnSig::empty(),
        body,
        is_async: false,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    }
}
