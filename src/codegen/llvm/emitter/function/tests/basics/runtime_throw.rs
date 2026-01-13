use super::super::helpers::{function_ir, test_target};
use crate::chic_kind::ChicKind;
use crate::codegen::CodegenOptions;
use crate::codegen::llvm::emit_module;
use crate::codegen::llvm::signatures::build_signatures;
use crate::mir::{
    BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId,
    LocalKind, MirBody, MirFunction, MirModule, Operand, Place, Rvalue, Statement, StatementKind,
    Terminator, Ty,
};
use crate::perf::PerfMetadata;
use crate::runtime::exception_type_identity;

#[test]
fn llvm_emitter_calls_runtime_throw_with_type_identity() {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("payload".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0x1234)))),
        },
    });
    entry.terminator = Some(Terminator::Throw {
        exception: Some(Operand::Copy(Place::new(LocalId(1)))),
        ty: Some(Ty::named("Demo::Failure")),
    });
    body.blocks.push(entry);

    let function = MirFunction {
        name: "Demo::Throw".into(),
        kind: FunctionKind::Function,
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
    .expect("emit IR");
    let body_ir = function_ir(&ir, "Demo__Throw");
    let type_id = exception_type_identity("Demo::Failure");
    assert!(
        body_ir.contains("@chic_rt_throw"),
        "expected throw terminator to invoke runtime hook:\n{body_ir}"
    );
    assert!(
        body_ir.contains(&format!("i64 {type_id}")),
        "expected throw to embed type identity {type_id}:\n{body_ir}"
    );
    assert!(
        ir.contains("declare void @chic_rt_throw(i64, i64)"),
        "expected module to declare chic_rt_throw hook:\n{ir}"
    );
}
