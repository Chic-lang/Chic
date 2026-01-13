use super::helpers::{function_ir, test_target};
use crate::chic_kind::ChicKind;
use crate::codegen::CodegenOptions;
use crate::codegen::llvm::emit_module;
use crate::codegen::llvm::signatures::{build_signatures, canonical_function_name};
use crate::codegen::llvm::types::constrained_rounding_string;
use crate::mir::{
    Abi, BasicBlock, BlockId, CastKind, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl,
    LocalId, LocalKind, MirBody, MirFunction, MirModule, Operand, Place, RoundingMode, Rvalue,
    Statement, StatementKind, Terminator, Ty,
};
use crate::perf::PerfMetadata;

#[test]
fn int_downcast_emits_trunc() {
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
                    operand: Operand::Const(ConstOperand::new(ConstValue::Int(42))),
                    source: Ty::named("int"),
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
        name: "Root::CastDown".into(),
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

    let mut module = MirModule::default();
    module.functions.push(function);
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let options = CodegenOptions::default();
    let perf = PerfMetadata::default();
    let ir = emit_module(
        &module,
        None,
        &perf,
        &signatures,
        None,
        ChicKind::StaticLibrary,
        target.triple(),
        &target,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("emit module");
    let canonical = canonical_function_name("Root::CastDown");
    let symbol = signatures
        .get(&canonical)
        .map(|sig| sig.symbol.clone())
        .expect("function symbol");
    let fn_ir = function_ir(&ir, &symbol);
    assert!(
        fn_ir.contains("trunc i32 42 to i8"),
        "expected trunc instruction, got:\n{}",
        fn_ir
    );
}

#[test]
fn int_to_float_emits_sitofp() {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("double"),
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
                    kind: CastKind::IntToFloat,
                    operand: Operand::Const(ConstOperand::new(ConstValue::Int(7))),
                    source: Ty::named("long"),
                    target: Ty::named("double"),
                    rounding: None,
                },
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    };
    body.blocks.push(entry);

    let function = MirFunction {
        name: "Root::CastToFloat".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named("double"),
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

    let mut module = MirModule::default();
    module.functions.push(function);
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let options = CodegenOptions::default();
    let perf = PerfMetadata::default();
    let ir = emit_module(
        &module,
        None,
        &perf,
        &signatures,
        None,
        ChicKind::StaticLibrary,
        target.triple(),
        &target,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("emit module");
    let canonical = canonical_function_name("Root::CastToFloat");
    let symbol = signatures
        .get(&canonical)
        .map(|sig| sig.symbol.clone())
        .expect("function symbol");
    let fn_ir = function_ir(&ir, &symbol);
    let rounding = constrained_rounding_string(RoundingMode::NearestTiesToEven);
    assert!(
        fn_ir.contains(&format!(
            "@llvm.experimental.constrained.sitofp.f64.i64(i64 7, metadata !\"{rounding}\", metadata !\"fpexcept.strict\")"
        )),
        "expected constrained sitofp instruction, got:\n{}",
        fn_ir
    );
}

#[test]
fn float_rounding_metadata_respects_mir_hint() {
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
                value: Rvalue::Cast {
                    kind: CastKind::FloatToFloat,
                    operand: Operand::Const(ConstOperand::new(ConstValue::Float(
                        crate::mir::FloatValue::f64_from_bits(0x3ff0_0000_0000_0000),
                    ))),
                    source: Ty::named("double"),
                    target: Ty::named("float"),
                    rounding: Some(RoundingMode::TowardZero),
                },
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    };
    body.blocks.push(entry);

    let function = MirFunction {
        name: "Root::RoundCast".into(),
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

    let mut module = MirModule::default();
    module.functions.push(function);
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let options = CodegenOptions::default();
    let perf = PerfMetadata::default();
    let ir = emit_module(
        &module,
        None,
        &perf,
        &signatures,
        None,
        ChicKind::StaticLibrary,
        target.triple(),
        &target,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("emit module");
    let canonical = canonical_function_name("Root::RoundCast");
    let symbol = signatures
        .get(&canonical)
        .map(|sig| sig.symbol.clone())
        .expect("function symbol");
    let fn_ir = function_ir(&ir, &symbol);
    let rounding = constrained_rounding_string(RoundingMode::TowardZero);
    assert!(
        fn_ir.contains("constrained.fptrunc")
            && fn_ir.contains(&format!("metadata !\"{rounding}\""))
            && fn_ir.contains("metadata !\"fpexcept.strict\""),
        "expected constrained cast with explicit rounding, got:\n{fn_ir}"
    );
}

#[test]
fn pointer_to_int_emits_ptrtoint() {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("usize"),
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
                    kind: CastKind::PointerToInt,
                    operand: Operand::Const(ConstOperand::new(ConstValue::Int(0))),
                    source: Ty::named("int*"),
                    target: Ty::named("usize"),
                    rounding: None,
                },
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    };
    body.blocks.push(entry);

    let function = MirFunction {
        name: "Root::PtrCast".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named("usize"),
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

    let mut module = MirModule::default();
    module.functions.push(function);
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let options = CodegenOptions::default();
    let perf = PerfMetadata::default();
    let ir = emit_module(
        &module,
        None,
        &perf,
        &signatures,
        None,
        ChicKind::StaticLibrary,
        target.triple(),
        &target,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("emit module");
    let canonical = canonical_function_name("Root::PtrCast");
    let symbol = signatures
        .get(&canonical)
        .map(|sig| sig.symbol.clone())
        .expect("function symbol");
    let fn_ir = function_ir(&ir, &symbol);
    assert!(
        fn_ir.contains("ptrtoint i8* 0 to i64")
            || fn_ir.contains("ptrtoint i8* 0 to i32")
            || fn_ir.contains("ptrtoint i8* inttoptr (i64 0 to i8*) to i64")
            || fn_ir.contains("ptrtoint i8* inttoptr (i64 0 to i8*) to i32"),
        "expected ptrtoint instruction, got:\n{}",
        fn_ir
    );
}
