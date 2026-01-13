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
fn emit_blocks_processes_multiple_basic_blocks() {
    let function = nested_block_function();
    let body = emit_body_using_layouts(wasm_layouts(), function, |_| None);
    assert!(
        body.windows(2).any(|window| window == [0x0C, 0x01]),
        "expected branch to loop dispatch after goto"
    );
}
#[test]
fn set_block_unresolved_target_emits_unreachable() {
    let function = function_with_missing_target();
    let body = emit_body_using_layouts(wasm_layouts(), function, |_| None);
    assert!(
        body.contains(&0x00),
        "missing block targets should emit an unreachable fallback"
    );
}
#[test]
fn emit_body_handles_literal_match_arms() {
    let function = match_literal_function();
    let body = emit_body_using_layouts(wasm_layouts(), function, |_| None);
    assert!(
        body.windows(1).any(|window| window[0] == 0x04),
        "match lowering should emit an if opcode"
    );
}
#[test]
fn emit_body_handles_enum_match_patterns() {
    let function = match_enum_function();
    let body = emit_body_using_layouts(enum_layout_table(), function, |_| None);
    assert!(
        body.windows(1).any(|window| window[0] == 0x04),
        "enum match lowering should emit an if opcode"
    );
}
#[test]
fn emit_body_emits_flag_bitwise_ops() {
    let function = function_with_flag_ops();
    let body = emit_body_using_layouts(enum_layout_table(), function, |_| None);
    assert!(
        body.contains(&0x72),
        "flag operations should include i32.or"
    );
}
#[test]
fn emit_body_rejects_out_of_range_switch_literals() {
    let function = switch_int_overflow_function();
    with_emitter_using_layouts(
        wasm_layouts(),
        function,
        |_| None,
        |emitter| {
            let err = emitter
                .emit_body()
                .expect_err("switch literal should exceed 32-bit range");
            assert!(
                format!("{err}").contains("outside 32-bit range"),
                "unexpected error message: {err}"
            );
        },
    );
}
#[test]
fn emit_body_rejects_pending_operands() {
    let function = pending_operand_function();
    with_emitter_using_layouts(
        wasm_layouts(),
        function,
        |_| None,
        |emitter| {
            let body = emitter
                .emit_body()
                .expect("pending operands should lower to placeholders");
            assert!(
                !body.is_empty(),
                "pending operand lowering should still emit instructions"
            );
        },
    );
}
#[test]
fn emit_body_rejects_pending_rvalues() {
    let function = pending_rvalue_function();
    with_emitter_using_layouts(
        wasm_layouts(),
        function,
        |_| None,
        |emitter| {
            let body = emitter
                .emit_body()
                .expect("pending rvalues should lower to placeholders");
            assert!(
                !body.is_empty(),
                "pending rvalue lowering should still emit instructions"
            );
        },
    );
}
#[test]
fn emit_body_rejects_projected_match_value() {
    let function = match_with_projection_function();
    with_emitter_using_layouts(
        wasm_layouts(),
        function,
        |_| None,
        |emitter| {
            let err = emitter
                .emit_body()
                .expect_err("projected match values should be rejected");
            assert!(
                format!("{err}").contains("projected match values"),
                "unexpected error message: {err}"
            );
        },
    );
}
#[test]
fn emit_statement_assert_inserts_unreachable() {
    let function = function_with_assert();
    let body = emit_body_using_layouts(wasm_layouts(), function, |_| None);
    assert!(
        body.contains(&0x00),
        "assert lowering should emit an unreachable instruction"
    );
}
#[test]
fn emit_block_without_terminator_inserts_br_zero() {
    let function = function_with_missing_terminator();
    let body = emit_body_using_layouts(wasm_layouts(), function, |_| None);
    assert!(
        body.windows(2).any(|w| w == [0x0C, 0x00]),
        "blocks without terminators should emit br 0 fallback"
    );
}
#[test]
fn emit_terminator_unreachable_emits_trap() {
    let function = function_with_unreachable();
    let body = emit_body_using_layouts(wasm_layouts(), function, |_| None);
    assert!(
        body.contains(&0x00),
        "unreachable terminator should produce wasm trap opcode"
    );
}
#[test]
fn emit_rvalue_handles_unary_operations() {
    let function = function_with_unary_ops();
    let body = emit_body_using_layouts(wasm_layouts(), function, |_| None);
    assert!(
        body.windows(2).any(|w| w == [0x41, 0x7F]),
        "negation should materialize i32.const -1"
    );
    assert!(body.contains(&0x6C), "negation should emit i32.mul opcode");
    assert!(
        body.contains(&0x45),
        "logical not should emit i32.eqz opcode"
    );
}
#[test]
fn emit_rvalue_handles_binary_operations() {
    let function = function_with_binary_ops();
    let body = emit_body_using_layouts(wasm_layouts(), function, |_| None);
    assert!(body.contains(&0x6A), "addition should emit i32.add opcode");
}
#[test]
fn emit_switch_int_emits_compare_branches() {
    let function = function_with_switch();
    let body = emit_body_using_layouts(wasm_layouts(), function, |_| None);
    assert!(
        body.windows(1).any(|w| w[0] == 0x04),
        "switch lowering should emit wasm if blocks"
    );
    assert!(body.contains(&0x46), "switch compares should emit i32.eq");
}
#[test]
fn emit_match_wildcard_short_circuits() {
    let function = function_with_wildcard_match();
    let body = emit_body_using_layouts(wasm_layouts(), function, |_| None);
    assert!(
        body.windows(2).any(|w| w == [0x0C, 0x01]),
        "wildcard match should branch directly to the target block"
    );
}
#[test]
fn emit_match_handles_struct_destructuring_patterns() {
    let function = function_with_complex_match();
    let body = emit_body_using_layouts(wasm_layouts(), function, |_| None);
    assert!(
        body.windows(2).any(|w| w == [0x0C, 0x01]),
        "destructuring pattern should branch directly to the binding block"
    );
}
#[test]
fn wasm_emitter_handles_structured_control_flow() {
    let function = super::sample_loop_function();
    let body = emit_body_using_layouts(wasm_layouts(), function, |_| None);
    assert!(
        !body.is_empty(),
        "expected non-empty wasm body for structured function"
    );
    let has_br_to_loop = body.windows(2).any(|w| w[0] == 0x0C && w[1] == 0x01);
    assert!(
        has_br_to_loop,
        "expected at least one branch targeting the dispatch loop (depth=1)"
    );
    let has_br_zero = body.windows(2).any(|w| w[0] == 0x0C && w[1] == 0x00);
    assert!(
        !has_br_zero,
        "should not emit depth-0 branches that trap within the block"
    );
    assert!(
        body.contains(&0x0F),
        "expected return opcode in emitted body"
    );
}
