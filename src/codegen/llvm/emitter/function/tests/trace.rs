use std::collections::{BTreeSet, HashMap, HashSet};

use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::function::builder::emit_function;
use crate::codegen::llvm::emitter::literals::StrLiteralInfo;
use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
use crate::codegen::llvm::signatures::build_signatures;
use crate::mir::{
    Abi, BasicBlock, BlockId, FnSig, FunctionKind, LocalDecl, LocalKind, MirBody, MirFunction,
    MirModule, Terminator, Ty, TypeLayoutTable,
};
use crate::perf::{TraceLevel, Tracepoint, trace_id};
use crate::target::Target;

#[test]
fn emits_trace_enter_and_exit_calls() {
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

    let function = MirFunction {
        name: "Demo::trace_me".into(),
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

    let mut module = MirModule::default();
    module.type_layouts = TypeLayoutTable::default();
    module.functions.push(function.clone());

    let target = Target::parse("x86_64-unknown-linux-gnu").expect("target");
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let sig = signatures
        .get(&function.name)
        .expect("signature exists")
        .clone();
    let tracepoint = Tracepoint {
        function: function.name.clone(),
        label: "trace".into(),
        label_id: Some(crate::mir::StrId::new(0)),
        level: TraceLevel::Perf,
        trace_id: trace_id(&function.name, "trace"),
        span: None,
        budget: None,
    };

    let mut out = String::new();
    let mut externals = BTreeSet::new();
    let mut metadata = MetadataRegistry::new();
    let mut str_literals = HashMap::new();
    str_literals.insert(
        crate::mir::StrId::new(0),
        StrLiteralInfo {
            global: "@.str.trace".into(),
            array_len: 6,
            data_len: 5,
        },
    );

    emit_function(
        &mut out,
        &function,
        &sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut externals,
        &HashSet::new(),
        &[],
        &[],
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        &[],
        &str_literals,
        &module.type_layouts,
        &mut metadata,
        Some(&tracepoint),
    )
    .expect("emit trace");

    assert!(
        out.contains("chic_rt_trace_enter"),
        "trace enter call should be present: {out}"
    );
    assert!(
        out.contains("chic_rt_trace_exit"),
        "trace exit call should be present: {out}"
    );
}
