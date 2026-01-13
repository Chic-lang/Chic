use super::helpers::{function_ir, test_target};
use crate::chic_kind::ChicKind;
use crate::codegen::CodegenOptions;
use crate::codegen::llvm::emit_module;
use crate::codegen::llvm::signatures::build_signatures;
use crate::frontend::attributes::OptimizationHints;
use crate::mir::{
    Abi, BasicBlock, BlockId, FnSig, FunctionKind, LocalDecl, LocalKind, MirBody, MirFunction,
    MirModule, Terminator, Ty,
};
use crate::perf::PerfMetadata;

fn empty_body() -> MirBody {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    });
    body
}

fn make_function(name: &str, optimization_hints: OptimizationHints) -> MirFunction {
    MirFunction {
        name: name.into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        },
        body: empty_body(),
        is_async: false,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints,
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    }
}

fn emit_ir(functions: Vec<MirFunction>) -> String {
    let mut module = MirModule::default();
    module.functions = functions;
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let options = CodegenOptions::default();
    let perf = PerfMetadata::default();

    emit_module(
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
    .expect("emit module")
}

#[test]
fn emits_hot_and_always_inline_attributes() {
    let mut hints = OptimizationHints::default();
    hints.hot = true;
    hints.always_inline = true;
    let ir = emit_ir(vec![make_function("Root::HotInline", hints)]);
    let func_ir = function_ir(&ir, "Root__HotInline__baseline");

    assert!(
        func_ir.contains("@Root__HotInline__baseline()"),
        "expected mangled symbol, saw {func_ir:?}"
    );
    assert!(
        func_ir.contains("hot"),
        "expected hot attribute, saw {func_ir:?}"
    );
    assert!(
        func_ir.contains("alwaysinline"),
        "expected alwaysinline attribute, saw {func_ir:?}"
    );
    assert!(
        !func_ir.contains("cold"),
        "hot function should not be marked cold"
    );
    assert!(
        !func_ir.contains("noinline"),
        "alwaysinline should suppress noinline"
    );
}

#[test]
fn emits_cold_and_never_inline_attributes() {
    let mut hints = OptimizationHints::default();
    hints.cold = true;
    hints.never_inline = true;
    let ir = emit_ir(vec![make_function("Root::ColdNoInline", hints)]);
    let func_ir = function_ir(&ir, "Root__ColdNoInline__baseline");

    assert!(
        func_ir.contains("@Root__ColdNoInline__baseline()"),
        "expected mangled symbol, saw {func_ir:?}"
    );
    assert!(
        func_ir.contains("cold"),
        "expected cold attribute, saw {func_ir:?}"
    );
    assert!(
        func_ir.contains("noinline"),
        "expected noinline attribute, saw {func_ir:?}"
    );
    assert!(
        !func_ir.contains("hot"),
        "cold function should not be marked hot"
    );
    assert!(
        !func_ir.contains("alwaysinline"),
        "never_inline should suppress alwaysinline"
    );
}
