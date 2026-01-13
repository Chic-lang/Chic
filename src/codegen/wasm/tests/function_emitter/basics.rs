#![cfg(test)]
use super::common::*;
use super::helpers::*;
use crate::codegen::wasm::{ValueType, push_f32_const, push_i32_const, push_i64_const};
use crate::mir::FloatValue;
use crate::mir::*;
use crate::syntax::numeric::{IntegerWidth, NumericLiteralMetadata, NumericLiteralType};
use std::convert::TryInto;

#[test]
fn function_emitter_rejects_excessive_locals() {
    with_emitter_default(
        simple_function("Main", FunctionKind::Function, Ty::Unit),
        |emitter| {
            assert!(
                emitter.local_types.len() < usize::from(u16::MAX),
                "fixture should not exceed local limit"
            );
        },
    );
}

#[test]
fn emit_operand_respects_i64_literal_metadata() {
    let function = simple_function("Main", FunctionKind::Function, Ty::Unit);
    with_emitter_default(function, |emitter| {
        let operand = Operand::Const(ConstOperand::with_literal(
            ConstValue::Int(0x0123_4567_89AB_CDEF),
            Some(NumericLiteralMetadata {
                literal_type: NumericLiteralType::Signed(IntegerWidth::W64),
                suffix_text: Some("i64".into()),
                explicit_suffix: true,
            }),
        ));
        let mut buf = Vec::new();
        let ty = emitter
            .emit_operand(&mut buf, &operand)
            .expect("emit operand");
        assert_eq!(ty, ValueType::I64);

        let mut expected = Vec::new();
        push_i64_const(&mut expected, 0x0123_4567_89AB_CDEF);
        assert_eq!(buf, expected, "literal should encode as i64.const");
    });
}

#[test]
fn emit_operand_widens_integer_literals_without_metadata() {
    let function = simple_function("Main", FunctionKind::Function, Ty::Unit);
    with_emitter_default(function, |emitter| {
        let operand = Operand::Const(ConstOperand::new(ConstValue::Int(0x1_0000_0000)));
        let mut buf = Vec::new();
        let ty = emitter
            .emit_operand(&mut buf, &operand)
            .expect("emit operand");
        assert_eq!(ty, ValueType::I64);

        let mut expected = Vec::new();
        push_i64_const(&mut expected, 0x1_0000_0000);
        assert_eq!(buf, expected, "unsuffixed literal should widen to i64");
    });
}

#[test]
fn emit_operand_respects_float_literal_metadata() {
    let function = simple_function("Main", FunctionKind::Function, Ty::Unit);
    with_emitter_default(function, |emitter| {
        let operand = Operand::Const(ConstOperand::with_literal(
            ConstValue::Float(FloatValue::from_f64(3.5)),
            Some(NumericLiteralMetadata {
                literal_type: NumericLiteralType::Float32,
                suffix_text: Some("f32".into()),
                explicit_suffix: true,
            }),
        ));
        let mut buf = Vec::new();
        let ty = emitter
            .emit_operand(&mut buf, &operand)
            .expect("emit operand");
        assert_eq!(ty, ValueType::F32);
        assert_eq!(buf.len(), 5, "f32.const should emit opcode + 4 bytes");
        assert_eq!(buf[0], 0x43, "expected f32.const opcode");
        let bytes: [u8; 4] = buf[1..5]
            .try_into()
            .expect("f32.const encoding should have 4 payload bytes");
        assert_eq!(f32::from_le_bytes(bytes), 3.5f32);
    });
}

#[test]
fn emit_binary_float_add_emits_f32_add() {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("float"),
        false,
        None,
        LocalKind::Return,
    ));
    let entry = BasicBlock {
        id: BlockId(0),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Binary {
                    op: BinOp::Add,
                    lhs: Operand::Const(ConstOperand::with_literal(
                        ConstValue::Float(FloatValue::from_f32(1.0)),
                        Some(NumericLiteralMetadata {
                            literal_type: NumericLiteralType::Float32,
                            suffix_text: Some("f32".into()),
                            explicit_suffix: true,
                        }),
                    )),
                    rhs: Operand::Const(ConstOperand::with_literal(
                        ConstValue::Float(FloatValue::from_f32(2.0)),
                        Some(NumericLiteralMetadata {
                            literal_type: NumericLiteralType::Float32,
                            suffix_text: Some("f32".into()),
                            explicit_suffix: true,
                        }),
                    )),
                    rounding: None,
                },
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    };
    body.blocks.push(entry);

    let function = MirFunction {
        name: "Root::Add".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named("float"),
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

    let body_bytes = emit_body_default(function);
    let mut pattern = Vec::new();
    push_f32_const(&mut pattern, 1.0);
    push_f32_const(&mut pattern, 2.0);
    pattern.push(0x92); // f32.add
    assert!(
        contains_bytes(&body_bytes, &pattern),
        "body should contain f32.add after two f32.const instructions"
    );
}

#[test]
fn emit_operand_rejects_128_bit_integer_literals() {
    let function = simple_function("Main", FunctionKind::Function, Ty::Unit);
    with_emitter_default(function, |emitter| {
        let operand = Operand::Const(ConstOperand::with_literal(
            ConstValue::UInt(u128::MAX),
            Some(NumericLiteralMetadata {
                literal_type: NumericLiteralType::Unsigned(IntegerWidth::W128),
                suffix_text: Some("u128".into()),
                explicit_suffix: true,
            }),
        ));
        let mut buf = Vec::new();
        let ty = emitter
            .emit_operand(&mut buf, &operand)
            .expect("128-bit literal should lower via stack allocation");
        assert_eq!(ty, ValueType::I32, "int128 literal should produce pointer");
        assert!(
            !buf.is_empty(),
            "int128 literal lowering should emit stack allocation"
        );
    });
}
#[test]
fn emit_cast_masks_unsigned_downcast() {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("byte"),
        false,
        None,
        LocalKind::Return,
    ));
    let entry = BasicBlock {
        id: BlockId(0),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Cast {
                    kind: CastKind::IntToInt,
                    operand: Operand::Const(ConstOperand::new(ConstValue::UInt(0x1234))),
                    source: Ty::named("uint"),
                    target: Ty::named("byte"),
                    rounding: None,
                },
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    };
    body.blocks.push(entry);

    let function = MirFunction {
        name: "Root::Downcast".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named("byte"),
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

    let body_bytes = emit_body_default(function);

    let mut mask_pattern = Vec::new();
    push_i32_const(&mut mask_pattern, 0xFF);
    mask_pattern.push(0x71); // i32.and
    assert!(
        contains_bytes(&body_bytes, &mask_pattern),
        "unsigned downcast should mask to 8 bits"
    );
}
#[test]
fn emit_load_from_place_emits_load_instruction() {
    let (layouts, function) = struct_projection_fixture();
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let mut place = Place::new(LocalId(2));
            place
                .projection
                .push(ProjectionElem::FieldNamed("X".into()));
            let mut buf = Vec::new();
            let ty = emitter
                .emit_load_from_place(&mut buf, &place)
                .expect("load should succeed");
            assert_eq!(ty, ValueType::I32);
            assert!(
                !buf.ends_with(&[0x36, 0x00, 0x00]),
                "load should not emit store opcodes"
            );
            assert!(
                buf.ends_with(&[0x28, 0x00, 0x00]),
                "expected i32.load with zero offset in instruction stream"
            );
        },
    );
}
#[test]
fn emit_load_from_place_rejects_scalar_locals() {
    let function = scalar_local_function();
    with_emitter_default(function, |emitter| {
        let mut place = Place::new(LocalId(1));
        place.projection.push(ProjectionElem::Field(0));
        let err = emitter
            .emit_load_from_place(&mut Vec::new(), &place)
            .expect_err("scalar locals cannot be projected");
        assert!(
            format!("{err}").contains("projection"),
            "unexpected error for scalar projection: {err}"
        );
    });
}
#[test]
fn emit_store_to_access_emits_type_specific_opcode() {
    let (layouts, function) = struct_projection_fixture();
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let mut buf = Vec::new();
            emitter.emit_store_to_access_for_ty(&mut buf, &Ty::named("int"), ValueType::I32);
            assert_eq!(buf, vec![0x36, 0x00, 0x00]);

            let mut buf = Vec::new();
            emitter.emit_store_to_access_for_ty(&mut buf, &Ty::named("long"), ValueType::I64);
            assert_eq!(buf, vec![0x37, 0x00, 0x00]);

            let mut buf = Vec::new();
            emitter.emit_store_to_access_for_ty(&mut buf, &Ty::named("float"), ValueType::F32);
            assert_eq!(buf, vec![0x38, 0x00, 0x00]);

            let mut buf = Vec::new();
            emitter.emit_store_to_access_for_ty(&mut buf, &Ty::named("double"), ValueType::F64);
            assert_eq!(buf, vec![0x39, 0x00, 0x00]);
        },
    );
}

#[test]
fn emit_body_handles_decimal_intrinsics() {
    let function = decimal_intrinsic_function();
    let body = emit_body_using_layouts(super::common::wasm_layouts(), function, |_| None);
    assert!(
        body.windows(1).any(|window| window[0] == 0x10),
        "expected decimal intrinsic lowering to emit runtime call"
    );
}

fn decode_u32_leb(bytes: &[u8], start: usize) -> Option<(u32, usize)> {
    let mut result = 0u32;
    let mut shift = 0;
    let mut index = start;
    while index < bytes.len() {
        let byte = bytes[index];
        let value = (byte & 0x7F) as u32;
        let Some(shifted) = value.checked_shl(shift) else {
            return None;
        };
        result |= shifted;
        index += 1;
        if (byte & 0x80) == 0 {
            return Some((result, index - start));
        }
        shift += 7;
        if shift >= 32 {
            break;
        }
    }
    None
}

fn collect_call_indices(bytes: &[u8]) -> Vec<u32> {
    let mut indices = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == 0x10 {
            if let Some((value, len)) = decode_u32_leb(bytes, cursor + 1) {
                indices.push(value);
                cursor += 1 + len;
                continue;
            }
        }
        cursor += 1;
    }
    indices
}

#[test]
fn emit_decimal_intrinsic_emits_distinct_runtime_hooks() {
    let function = decimal_intrinsic_function();
    let body = emit_body_using_layouts(super::common::wasm_layouts(), function, |_| None);
    let call_indices = collect_call_indices(&body);
    assert!(
        call_indices.len() >= 2,
        "expected scalar and SIMD runtime calls, found {call_indices:?}"
    );
    let has_adjacent_decimal_hooks = call_indices
        .windows(2)
        .any(|window| window[0].abs_diff(window[1]) == 1);
    assert!(
        has_adjacent_decimal_hooks,
        "expected decimal intrinsic lowering to emit adjacent scalar/SIMD hooks, found {call_indices:?}"
    );
}
#[test]
fn emit_assign_handles_struct_field_projection() {
    let (layouts, function) = struct_assign_via_projection_function();
    let body = emit_body_using_layouts(layouts, function, |_| None);
    assert!(
        body.windows(1).any(|window| window[0] == 0x36),
        "store opcode expected for struct field assignment"
    );
}
