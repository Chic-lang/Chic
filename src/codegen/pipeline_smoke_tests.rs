use super::llvm::{build_signatures, emit_module as emit_llvm_module, find_entry_function};
use super::wasm::compile as compile_wasm;
use super::{Backend, CodegenArtifact, CodegenOptions};
use crate::ChicKind;
use crate::frontend::ast::Module as AstModule;
use crate::frontend::lexer::lex;
use crate::frontend::parser::parse_module;
use crate::mir::{
    Abi, AutoTraitOverride, AutoTraitSet, BasicBlock, BlockId, ConstGenericArg, FieldLayout, FnSig,
    FunctionKind, GenericArg, LocalDecl, LocalKind, MirFunction, MirModule, PositionalElement,
    StructLayout, Terminator, Ty, TypeLayout, TypeRepr, borrow_check_module, lower_module,
    new_mir_body,
};
use crate::perf::PerfMetadata;
use crate::target::Target;
use crate::typeck::check_module;
use tempfile::tempdir;

const SMOKE_SOURCE: &str = r#"
namespace Smoke;

public int Main()
{
    if (Add(21, 21) == 42)
    {
        return 0;
    }

    return 1;
}

public int Add(int left, int right)
{
    return left + right;
}
"#;

struct PipelineArtifacts {
    ast: AstModule,
    mir: MirModule,
    entry: Option<String>,
}

fn run_pipeline(source: &str) -> PipelineArtifacts {
    let lex_output = lex(source);
    assert!(
        lex_output.diagnostics.is_empty(),
        "lexer diagnostics: {:?}",
        lex_output.diagnostics
    );

    let parse = parse_module(source).expect("module parses");
    assert!(
        parse.diagnostics.is_empty(),
        "parser diagnostics: {:?}",
        parse.diagnostics
    );
    let ast = parse.module_owned();

    let lowering = lower_module(&ast);
    assert!(
        lowering.diagnostics.is_empty(),
        "lowering diagnostics: {:?}",
        lowering.diagnostics
    );

    let typecheck = check_module(&ast, &lowering.constraints, &lowering.module.type_layouts);
    assert!(
        typecheck.diagnostics.is_empty(),
        "typechecker diagnostics: {:?}",
        typecheck.diagnostics
    );

    let borrow = borrow_check_module(&lowering.module);
    assert!(
        borrow.diagnostics.is_empty(),
        "borrow checker diagnostics: {:?}",
        borrow.diagnostics
    );

    let entry = find_entry_function(&ast);

    PipelineArtifacts {
        ast,
        mir: lowering.module,
        entry,
    }
}

fn append_const_generic_fixture(mir: &mut MirModule) {
    let mut fixed_ty = Ty::named("Smoke::Fixed");
    if let Some(named) = fixed_ty.as_named_mut() {
        named
            .args
            .push(GenericArg::Const(ConstGenericArg::new("4")));
    }

    if !mir.type_layouts.types.contains_key("Smoke::Fixed") {
        let field = FieldLayout {
            name: "Value".into(),
            ty: Ty::named("int"),
            index: 0,
            offset: Some(0),
            span: None,
            mmio: None,
            display_name: Some("Value".into()),
            is_required: false,
            is_nullable: false,
            is_readonly: false,

            view_of: None,
        };
        let layout = StructLayout {
            name: "Smoke::Fixed".into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: vec![field],
            positional: vec![PositionalElement {
                field_index: 0,
                name: Some("Value".into()),
                span: None,
            }],
            list: None,
            size: Some(4),
            align: Some(4),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        };
        mir.type_layouts
            .types
            .insert("Smoke::Fixed".into(), TypeLayout::Struct(layout));
    }

    let mut body = new_mir_body(1, None);
    body.locals.push(LocalDecl::new(
        None,
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("input".into()),
        fixed_ty.clone(),
        false,
        None,
        LocalKind::Arg(0),
    ));
    let mut block = BasicBlock::new(BlockId(0), None);
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    let function = MirFunction {
        name: "Smoke::UseFixed".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![fixed_ty],
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
    mir.functions.push(function);
}

#[test]
fn pipeline_smoke_generates_llvm_ir() {
    let PipelineArtifacts { ast, mir, entry } = run_pipeline(SMOKE_SOURCE);

    let target = Target::parse("x86_64-unknown-none").expect("parse target");
    let signatures =
        build_signatures(&mir, entry.as_deref(), &target).expect("build llvm signatures");

    let mut options = CodegenOptions::default();
    options.backend = Backend::Llvm;
    options.keep_object = false;
    options.link_final_artifact = false;
    let perf = PerfMetadata::default();

    let ir = emit_llvm_module(
        &mir,
        None,
        &perf,
        &signatures,
        entry.as_deref(),
        ChicKind::Executable,
        target.triple(),
        &target,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("emit llvm module");

    assert!(
        ir.contains("target triple"),
        "expected IR to contain target triple annotation"
    );
    assert!(
        ir.contains("@Smoke__Add"),
        "expected IR to contain sanitized symbol for helper function:\n{ir}"
    );
    assert!(
        signatures.contains_key("Smoke::Main"),
        "entry signature missing from LLVM signature table"
    );

    assert!(
        !ast.items.is_empty(),
        "pipeline AST should contain items for downstream stages"
    );
}

#[test]
fn pipeline_smoke_generates_wasm_module() {
    let PipelineArtifacts { ast, mir, .. } = run_pipeline(SMOKE_SOURCE);
    let target = Target::parse("x86_64-unknown-none").expect("parse target");

    let temp_dir = tempdir().expect("create temp dir");
    let output_path = temp_dir.path().join("smoke.wasm");

    let mut options = CodegenOptions::default();
    options.backend = Backend::Wasm;
    options.keep_object = true;
    options.link_final_artifact = false;
    let perf = PerfMetadata::default();

    let artifact: CodegenArtifact = compile_wasm(
        &ast,
        &mir,
        &perf,
        &target,
        ChicKind::Executable,
        &output_path,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("emit wasm module");

    let metadata =
        std::fs::metadata(&artifact.artifact_path).expect("wasm artifact should exist on disk");
    assert!(
        metadata.len() > 8,
        "expected wasm artifact to be non-empty (len = {})",
        metadata.len()
    );
}

#[test]
fn pipeline_handles_const_generics_across_backends() {
    let PipelineArtifacts {
        ast,
        mut mir,
        entry,
    } = run_pipeline(SMOKE_SOURCE);
    append_const_generic_fixture(&mut mir);

    let fixture = mir
        .functions
        .iter()
        .find(|func| func.name.ends_with("::UseFixed"))
        .expect("const generic function should be lowered");
    assert!(
        fixture
            .signature
            .params
            .iter()
            .any(|ty| ty.canonical_name().contains("<4>")),
        "expected const argument in parameter type"
    );
    assert!(
        fixture.name.contains("UseFixed"),
        "expected fixture function to be present"
    );

    let target = Target::parse("x86_64-unknown-none").expect("parse target");
    let signatures = build_signatures(&mir, entry.as_deref(), &target)
        .expect("build llvm signatures for const generics");

    let mut llvm_options = CodegenOptions::default();
    llvm_options.backend = Backend::Llvm;
    llvm_options.keep_object = false;
    llvm_options.link_final_artifact = false;
    let perf = PerfMetadata::default();

    let ir = emit_llvm_module(
        &mir,
        None,
        &perf,
        &signatures,
        entry.as_deref(),
        ChicKind::Executable,
        target.triple(),
        &target,
        &llvm_options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("emit llvm module with const generics");
    assert!(
        ir.contains("Smoke__UseFixed"),
        "expected IR to contain symbol for const generic function:\n{ir}"
    );

    let temp_dir = tempdir().expect("create temp dir");
    let wasm_path = temp_dir.path().join("const_generic.wasm");

    let mut wasm_options = CodegenOptions::default();
    wasm_options.backend = Backend::Wasm;
    wasm_options.keep_object = true;
    wasm_options.link_final_artifact = false;

    let artifact: CodegenArtifact = compile_wasm(
        &ast,
        &mir,
        &perf,
        &target,
        ChicKind::Executable,
        &wasm_path,
        &wasm_options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("emit wasm module with const generics");
    assert!(
        artifact.artifact_path.exists(),
        "wasm artifact for const generics should exist"
    );
}
