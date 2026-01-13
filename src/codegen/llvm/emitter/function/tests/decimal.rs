use super::helpers::{function_ir, test_target};
use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::function::builder::emit_function;
use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
use crate::codegen::llvm::signatures::build_signatures;
use crate::decimal::Decimal128;
use crate::mir::{
    Abi, BasicBlock, BlockId, ConstOperand, ConstValue, DecimalIntrinsic, DecimalIntrinsicKind,
    FnSig, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody, MirFunction, MirModule, Operand,
    Place, Rvalue, Statement, StatementKind, Terminator, Ty,
};
use std::collections::{BTreeSet, HashMap, HashSet};

const DECIMAL_RESULT: &str = "Std::Numeric::Decimal::DecimalIntrinsicResult";
const DECIMAL_VECTORIZE: &str = "Std::Numeric::Decimal::DecimalVectorizeHint";
const ROUNDING_MODE: &str = "Std::Numeric::Decimal::DecimalRoundingMode";
const DECIMAL_RUNTIME_CALL: &str = "Std::Numeric::Decimal::DecimalRuntimeCall";

#[test]
fn decimal_add_with_options_emits_scalar_and_simd_paths() {
    let module = decimal_with_options_module("Std::Decimal::Intrinsics::AddWithOptions");
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("decimal signatures");
    let function = module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::CallAdd"))
        .expect("decimal caller function");
    let sig = signatures
        .get(&function.name)
        .expect("caller signature present");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        &module.trait_vtables,
        &module.class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit decimal add caller");

    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("@chic_rt_decimal_add("),
        "expected scalar runtime call when vectorise hint is dynamic: {body}"
    );
    assert!(
        !body.contains("@chic_rt_decimal_add_simd("),
        "simd runtime call should not be emitted; SIMD path is routed through scalar runtime: {body}"
    );
}

#[test]
fn decimal_add_vectorized_prefers_simd_runtime_only() {
    let module = decimal_vectorized_module("Std::Decimal::Intrinsics::AddVectorized");
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("decimal signatures");
    let function = module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::CallAddVectorized"))
        .expect("decimal caller function");
    let sig = signatures
        .get(&function.name)
        .expect("caller signature present");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        &module.trait_vtables,
        &module.class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit decimal add vectorized caller");

    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("@chic_rt_decimal_add("),
        "vectorized wrapper should call scalar runtime until SIMD is Chic-native: {body}"
    );
    assert!(
        !body.contains("@chic_rt_decimal_add_simd("),
        "SIMD runtime symbol should not be referenced: {body}"
    );
}

#[test]
fn decimal_add_scalar_wrapper_calls_scalar_runtime() {
    let module = decimal_scalar_module("Std::Decimal::Intrinsics::Add");
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("decimal signatures");
    let function = module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::CallAddScalar"))
        .expect("decimal caller function");
    let sig = signatures
        .get(&function.name)
        .expect("caller signature present");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        &module.trait_vtables,
        &module.class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit decimal add scalar caller");

    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("@chic_rt_decimal_add("),
        "scalar wrapper should call scalar runtime directly: {body}"
    );
    assert!(
        !body.contains("@chic_rt_decimal_add_simd("),
        "scalar wrapper should not issue SIMD runtime call: {body}"
    );
}

#[test]
fn decimal_fma_wrapper_calls_scalar_runtime() {
    let module = decimal_fma_wrapper_module("Std::Decimal::Intrinsics::Fma");
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("decimal signatures");
    let function = module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::CallFmaScalar"))
        .expect("decimal caller function");
    let sig = signatures
        .get(&function.name)
        .expect("caller signature present");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        &module.trait_vtables,
        &module.class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit decimal fma scalar caller");

    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("@chic_rt_decimal_fma("),
        "scalar fma wrapper should call scalar runtime directly: {body}"
    );
    assert!(
        !body.contains("@chic_rt_decimal_fma_simd("),
        "scalar fma wrapper should avoid simd runtime: {body}"
    );
}

#[test]
fn decimal_fma_vectorized_wrapper_calls_simd_runtime_only() {
    let module = decimal_fma_wrapper_module("Std::Decimal::Intrinsics::FmaVectorized");
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("decimal signatures");
    let function = module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::CallFmaVectorized"))
        .expect("decimal caller function");
    let sig = signatures
        .get(&function.name)
        .expect("caller signature present");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        &module.trait_vtables,
        &module.class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit decimal fma vectorized caller");

    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("@chic_rt_decimal_fma("),
        "vectorized fma wrapper should call scalar runtime while SIMD path is disabled: {body}"
    );
    assert!(
        !body.contains("@chic_rt_decimal_fma_simd("),
        "SIMD runtime symbol should not be referenced: {body}"
    );
}

#[test]
fn decimal_add_vectorized_with_rounding_prefers_simd_runtime_only() {
    let module = decimal_vectorized_with_rounding_module(
        "Std::Decimal::Intrinsics::AddVectorizedWithRounding",
    );
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("decimal signatures");
    let function = module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::CallAddVectorizedRounding"))
        .expect("decimal caller function");
    let sig = signatures
        .get(&function.name)
        .expect("caller signature present");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        &module.trait_vtables,
        &module.class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit decimal add vectorized rounding caller");

    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("@chic_rt_decimal_add("),
        "vectorized rounding wrapper should call scalar runtime while SIMD path is disabled: {body}"
    );
    assert!(
        !body.contains("@chic_rt_decimal_add_simd("),
        "SIMD runtime symbol should not be referenced: {body}"
    );
}

#[test]
fn decimal_intrinsic_const_vectorize_false_emits_scalar_path() {
    let module =
        decimal_intrinsic_module("Demo::Decimal::IntrinsicScalar", vectorize_operand(false));
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("decimal signatures");
    let function = module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::IntrinsicScalar"))
        .expect("decimal intrinsic function");
    let sig = signatures
        .get(&function.name)
        .expect("caller signature present");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        &module.trait_vtables,
        &module.class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit decimal intrinsic scalar");

    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("@chic_rt_decimal_add("),
        "constant none vectorize hint should stick to scalar runtime: {body}"
    );
    assert!(
        !body.contains("@chic_rt_decimal_add_simd("),
        "constant none vectorize hint should not emit simd runtime: {body}"
    );
    assert!(
        !body.contains("select i1"),
        "constant path should not select between scalar and simd: {body}"
    );
}

#[test]
fn decimal_intrinsic_const_vectorize_true_emits_scalar_path_for_now() {
    let module = decimal_intrinsic_module("Demo::Decimal::IntrinsicSimd", vectorize_operand(true));
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("decimal signatures");
    let function = module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::IntrinsicSimd"))
        .expect("decimal intrinsic function");
    let sig = signatures
        .get(&function.name)
        .expect("caller signature present");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        &module.trait_vtables,
        &module.class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit decimal intrinsic simd");

    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("@chic_rt_decimal_add("),
        "constant vectorize decimal hint should currently lower to scalar runtime: {body}"
    );
    assert!(
        !body.contains("@chic_rt_decimal_add_simd("),
        "SIMD runtime symbol should not be referenced: {body}"
    );
}

#[test]
fn decimal_fma_with_rounding_emits_simd_path() {
    let module = decimal_fma_module("Std::Decimal::Intrinsics::FmaWithRounding");
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("decimal signatures");
    let function = module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::CallFma"))
        .expect("decimal caller function");
    let sig = signatures
        .get(&function.name)
        .expect("caller signature present");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        &module.trait_vtables,
        &module.class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit decimal fma caller");

    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("@chic_rt_decimal_fma("),
        "fma with rounding should call scalar runtime while SIMD is disabled: {body}"
    );
    assert!(
        !body.contains("@chic_rt_decimal_fma_simd("),
        "SIMD runtime symbol should not be referenced: {body}"
    );
}

#[test]
fn decimal_runtime_call_by_repr_invokes_runtime_symbol() {
    let module = decimal_runtime_module(
        "Std::Decimal::Intrinsics::RuntimeIntrinsics::Chic_Rt_Decimal_Div",
        "Demo::Decimal::CallRuntimeDiv",
    );
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("decimal signatures");
    let function = module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::CallRuntimeDiv"))
        .expect("runtime caller function");
    let sig = signatures
        .get(&function.name)
        .expect("caller signature present");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        &module.trait_vtables,
        &module.class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit decimal runtime caller");

    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("@chic_rt_decimal_div("),
        "runtime repr should route to decimal runtime symbol: {body}"
    );
}

#[test]
fn decimal_fma_with_options_honors_vectorize_operand() {
    let module = decimal_fma_module("Std::Decimal::Intrinsics::FmaWithOptions");
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("decimal signatures");
    let function = module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::CallFma"))
        .expect("decimal caller function");
    let sig = signatures
        .get(&function.name)
        .expect("caller signature present");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        &module.trait_vtables,
        &module.class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit decimal fma options caller");

    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("select i1"),
        "options path should emit selects when vectorize hint is dynamic: {body}"
    );
    assert!(
        body.contains("@chic_rt_decimal_fma("),
        "options path should include scalar runtime branch: {body}"
    );
    assert!(
        !body.contains("@chic_rt_decimal_fma_simd("),
        "SIMD runtime symbol should not be referenced: {body}"
    );
}

#[test]
fn decimal_wrappers_reject_missing_arguments() {
    let module = decimal_invalid_call_module(
        "Std::Decimal::Intrinsics::Add",
        vec![decimal_operand()],
        "Demo::Decimal::BadAdd",
        DECIMAL_RESULT,
    );
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("decimal signatures");
    let function = module.functions.first().expect("bad add function");
    let sig = signatures
        .get(&function.name)
        .expect("caller signature present");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    let err = emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        &module.trait_vtables,
        &module.class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect_err("wrapper should reject missing decimal operand");
    assert!(
        err.to_string().contains("expects two decimal arguments"),
        "unexpected error message: {err}"
    );

    let rounding_module = decimal_invalid_call_module(
        "Std::Decimal::Intrinsics::AddVectorizedWithRounding",
        vec![decimal_operand(), decimal_operand()],
        "Demo::Decimal::BadAddRounding",
        DECIMAL_RESULT,
    );
    let signatures = build_signatures(&rounding_module, None, &target).expect("decimal signatures");
    let function = rounding_module
        .functions
        .first()
        .expect("bad rounding function");
    let sig = signatures
        .get(&function.name)
        .expect("caller signature present");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    let err = emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        &rounding_module.trait_vtables,
        &rounding_module.class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        rounding_module.statics.as_slice(),
        &HashMap::new(),
        &rounding_module.type_layouts,
        &mut metadata,
        None,
    )
    .expect_err("wrapper should reject missing rounding operand");
    assert!(
        err.to_string()
            .contains("expects decimal, decimal, and rounding arguments"),
        "unexpected error message: {err}"
    );

    let options_module = decimal_invalid_call_module(
        "Std::Decimal::Intrinsics::AddWithOptions",
        vec![decimal_operand(), decimal_operand(), rounding_operand()],
        "Demo::Decimal::BadAddOptions",
        DECIMAL_RESULT,
    );
    let signatures = build_signatures(&options_module, None, &target).expect("decimal signatures");
    let function = options_module
        .functions
        .first()
        .expect("bad options function");
    let sig = signatures
        .get(&function.name)
        .expect("caller signature present");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    let err = emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        &options_module.trait_vtables,
        &options_module.class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        options_module.statics.as_slice(),
        &HashMap::new(),
        &options_module.type_layouts,
        &mut metadata,
        None,
    )
    .expect_err("wrapper should reject missing vectorize operand");
    assert!(
        err.to_string()
            .contains("expects decimal, decimal, rounding, and vectorize arguments"),
        "unexpected error message: {err}"
    );

    let fma_module = decimal_invalid_call_module(
        "Std::Decimal::Intrinsics::FmaWithOptions",
        vec![decimal_operand(), decimal_operand(), decimal_operand()],
        "Demo::Decimal::BadFma",
        DECIMAL_RESULT,
    );
    let signatures = build_signatures(&fma_module, None, &target).expect("decimal signatures");
    let function = fma_module.functions.first().expect("bad fma function");
    let sig = signatures
        .get(&function.name)
        .expect("caller signature present");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    let err = emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        &fma_module.trait_vtables,
        &fma_module.class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        fma_module.statics.as_slice(),
        &HashMap::new(),
        &fma_module.type_layouts,
        &mut metadata,
        None,
    )
    .expect_err("fma wrapper should reject missing rounding/vectorize operands");
    assert!(
        err.to_string()
            .contains("expects three decimal, rounding, and vectorize arguments"),
        "unexpected error message: {err}"
    );
}

fn decimal_with_options_module(symbol: &str) -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(decimal_with_options_fn(symbol));
    module
}

fn decimal_with_options_fn(symbol: &str) -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named(DECIMAL_RESULT),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("tmp".into()),
        Ty::named(DECIMAL_RESULT),
        false,
        None,
        LocalKind::Temp,
    ));
    body.locals.push(LocalDecl::new(
        Some("hint".into()),
        Ty::named(DECIMAL_VECTORIZE),
        true,
        None,
        LocalKind::Arg(0),
    ));

    let entry = body.entry();
    body.blocks.push(BasicBlock {
        id: entry,
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol.into()))),
            args: vec![
                decimal_operand(),
                decimal_operand(),
                rounding_operand(),
                Operand::Move(Place::new(LocalId(2))),
            ],
            arg_modes: Vec::new(),
            destination: Some(Place::new(LocalId(1))),
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Use(Operand::Move(Place::new(LocalId(1)))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });

    MirFunction {
        name: "Demo::Decimal::CallAdd".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::named(DECIMAL_VECTORIZE)],
            ret: Ty::named(DECIMAL_RESULT),
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
    }
}

fn decimal_vectorized_module(symbol: &str) -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(decimal_vectorized_fn(symbol));
    module
}

fn decimal_vectorized_with_rounding_module(symbol: &str) -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module
        .functions
        .push(decimal_vectorized_with_rounding_fn(symbol));
    module
}

fn decimal_vectorized_fn(symbol: &str) -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named(DECIMAL_RESULT),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("tmp".into()),
        Ty::named(DECIMAL_RESULT),
        false,
        None,
        LocalKind::Temp,
    ));

    let entry = body.entry();
    body.blocks.push(BasicBlock {
        id: entry,
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol.into()))),
            args: vec![decimal_operand(), decimal_operand()],
            arg_modes: Vec::new(),
            destination: Some(Place::new(LocalId(1))),
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Use(Operand::Move(Place::new(LocalId(1)))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });

    MirFunction {
        name: "Demo::Decimal::CallAddVectorized".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named(DECIMAL_RESULT),
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
    }
}

fn decimal_scalar_module(symbol: &str) -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(decimal_scalar_fn(symbol));
    module
}

fn decimal_scalar_fn(symbol: &str) -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named(DECIMAL_RESULT),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("tmp".into()),
        Ty::named(DECIMAL_RESULT),
        false,
        None,
        LocalKind::Temp,
    ));

    let entry = body.entry();
    body.blocks.push(BasicBlock {
        id: entry,
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol.into()))),
            args: vec![decimal_operand(), decimal_operand()],
            arg_modes: Vec::new(),
            destination: Some(Place::new(LocalId(1))),
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Use(Operand::Move(Place::new(LocalId(1)))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });

    MirFunction {
        name: "Demo::Decimal::CallAddScalar".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named(DECIMAL_RESULT),
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
    }
}

fn decimal_vectorized_with_rounding_fn(symbol: &str) -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named(DECIMAL_RESULT),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("tmp".into()),
        Ty::named(DECIMAL_RESULT),
        false,
        None,
        LocalKind::Temp,
    ));

    let entry = body.entry();
    body.blocks.push(BasicBlock {
        id: entry,
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol.into()))),
            args: vec![decimal_operand(), decimal_operand(), rounding_operand()],
            arg_modes: Vec::new(),
            destination: Some(Place::new(LocalId(1))),
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Use(Operand::Move(Place::new(LocalId(1)))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });

    MirFunction {
        name: "Demo::Decimal::CallAddVectorizedRounding".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named(DECIMAL_RESULT),
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
    }
}

fn decimal_operand() -> Operand {
    Operand::Const(ConstOperand::new(ConstValue::Decimal(Decimal128::zero())))
}

fn rounding_operand() -> Operand {
    Operand::Const(ConstOperand::new(ConstValue::Enum {
        type_name: ROUNDING_MODE.into(),
        variant: "TiesToEven".into(),
        discriminant: 0,
    }))
}

fn vectorize_operand(enabled: bool) -> Operand {
    let variant = if enabled { "Decimal" } else { "None" };
    Operand::Const(ConstOperand::new(ConstValue::Enum {
        type_name: DECIMAL_VECTORIZE.into(),
        variant: variant.into(),
        discriminant: if enabled { 1 } else { 0 },
    }))
}

fn decimal_fma_wrapper_module(symbol: &str) -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(decimal_fma_wrapper_fn(symbol));
    module
}

fn decimal_fma_wrapper_fn(symbol: &str) -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named(DECIMAL_RESULT),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("tmp".into()),
        Ty::named(DECIMAL_RESULT),
        false,
        None,
        LocalKind::Temp,
    ));

    let entry = body.entry();
    body.blocks.push(BasicBlock {
        id: entry,
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol.into()))),
            args: vec![decimal_operand(), decimal_operand(), decimal_operand()],
            arg_modes: Vec::new(),
            destination: Some(Place::new(LocalId(1))),
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Use(Operand::Move(Place::new(LocalId(1)))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });

    MirFunction {
        name: if symbol.ends_with("Vectorized") {
            "Demo::Decimal::CallFmaVectorized".into()
        } else {
            "Demo::Decimal::CallFmaScalar".into()
        },
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named(DECIMAL_RESULT),
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
    }
}

fn decimal_fma_module(symbol: &str) -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(decimal_fma_fn(symbol));
    module
}

fn decimal_fma_fn(symbol: &str) -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named(DECIMAL_RESULT),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("tmp".into()),
        Ty::named(DECIMAL_RESULT),
        false,
        None,
        LocalKind::Temp,
    ));
    body.locals.push(LocalDecl::new(
        Some("hint".into()),
        Ty::named(DECIMAL_VECTORIZE),
        true,
        None,
        LocalKind::Arg(0),
    ));

    let entry = body.entry();
    body.blocks.push(BasicBlock {
        id: entry,
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol.into()))),
            args: vec![
                decimal_operand(),
                decimal_operand(),
                decimal_operand(),
                rounding_operand(),
                Operand::Move(Place::new(LocalId(2))),
            ],
            arg_modes: Vec::new(),
            destination: Some(Place::new(LocalId(1))),
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Use(Operand::Move(Place::new(LocalId(1)))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });

    MirFunction {
        name: "Demo::Decimal::CallFma".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::named(DECIMAL_VECTORIZE)],
            ret: Ty::named(DECIMAL_RESULT),
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
    }
}

fn decimal_intrinsic_module(name: &str, vectorize: Operand) -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(decimal_intrinsic_fn(name, vectorize));
    module
}

fn decimal_intrinsic_fn(name: &str, vectorize: Operand) -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named(DECIMAL_RESULT),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("tmp".into()),
        Ty::named(DECIMAL_RESULT),
        false,
        None,
        LocalKind::Temp,
    ));

    let entry = body.entry();
    body.blocks.push(BasicBlock {
        id: entry,
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(1)),
                value: Rvalue::DecimalIntrinsic(DecimalIntrinsic {
                    kind: DecimalIntrinsicKind::Add,
                    lhs: decimal_operand(),
                    rhs: decimal_operand(),
                    addend: None,
                    rounding: rounding_operand(),
                    vectorize,
                }),
            },
        }],
        terminator: Some(Terminator::Goto { target: BlockId(1) }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Use(Operand::Move(Place::new(LocalId(1)))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });

    MirFunction {
        name: name.into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named(DECIMAL_RESULT),
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
    }
}

fn decimal_runtime_module(symbol: &str, name: &str) -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(decimal_runtime_fn(symbol, name));
    module
}

fn decimal_runtime_fn(symbol: &str, name: &str) -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named(DECIMAL_RUNTIME_CALL),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("tmp".into()),
        Ty::named(DECIMAL_RUNTIME_CALL),
        false,
        None,
        LocalKind::Temp,
    ));

    let entry = body.entry();
    body.blocks.push(BasicBlock {
        id: entry,
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol.into()))),
            args: vec![
                decimal_operand(),
                decimal_operand(),
                rounding_operand(),
                Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
            ],
            arg_modes: Vec::new(),
            destination: Some(Place::new(LocalId(1))),
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Use(Operand::Move(Place::new(LocalId(1)))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });

    MirFunction {
        name: name.into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named(DECIMAL_RUNTIME_CALL),
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
    }
}

fn decimal_invalid_call_module(
    symbol: &str,
    args: Vec<Operand>,
    name: &str,
    ret_ty: &str,
) -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();

    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named(ret_ty),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("tmp".into()),
        Ty::named(ret_ty),
        false,
        None,
        LocalKind::Temp,
    ));

    let entry = body.entry();
    body.blocks.push(BasicBlock {
        id: entry,
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol.into()))),
            args,
            arg_modes: Vec::new(),
            destination: Some(Place::new(LocalId(1))),
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Use(Operand::Move(Place::new(LocalId(1)))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });

    module.functions.push(MirFunction {
        name: name.into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named(ret_ty),
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
    });
    module
}
