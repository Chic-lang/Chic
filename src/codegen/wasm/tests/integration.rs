#![allow(unused_imports)]

use super::common::*;
use super::*;
use crate::chic_kind::ChicKind;
use crate::codegen::{Backend, CodegenOptions};
use crate::frontend::ast::{
    Block as AstBlock, CrateMainSetting, FunctionDecl, Item as AstItem, MemberDispatch,
    Module as AstModule, NamespaceDecl, Signature, TypeExpr, Visibility,
};
use crate::mir::{
    Abi, AutoTraitOverride, AutoTraitSet, BasicBlock, BinOp, BlockId, BorrowKind, BorrowOperand,
    ConstValue, EnumLayout, EnumVariantLayout, FieldLayout, FnSig, FunctionKind, LocalDecl,
    LocalId, LocalKind, MatchArm, MirBody, MirFunction, MirModule, Operand, Pattern,
    PatternBinding, PendingOperand, PendingRvalue, PendingTerminator, PendingTerminatorKind, Place,
    ProjectionElem, RegionVar, Rvalue, Statement, StatementKind, StructLayout, Terminator, Ty,
    TypeLayout, TypeLayoutTable, TypeRepr, UnOp, UnionFieldLayout, UnionFieldMode, UnionLayout,
    ValueCategory, VectorTy,
};
use crate::perf::PerfMetadata;
use crate::target::Target;
use std::collections::HashMap;
use std::fs;

#[test]
fn compile_requires_entry_for_executable() {
    let ast = AstModule::new(None);
    let mir = module_with_functions(Vec::new());
    let target = Target::default();
    let mut options = CodegenOptions::default();
    options.backend = Backend::Wasm;
    let perf = PerfMetadata::default();
    let output =
        std::env::temp_dir().join(format!("chic-test-missing-entry-{}", std::process::id()));
    let err = super::compile(
        &ast,
        &mir,
        &perf,
        &target,
        ChicKind::Executable,
        &output,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect_err("compile should error when Main entry is missing");
    assert!(
        format!("{err}").contains("Main"),
        "error should reference missing Main function: {err}"
    );
    if output.exists() {
        let _ = std::fs::remove_file(&output);
    }
}

#[test]
fn compile_allows_missing_entry_when_no_main() {
    let mut ast = AstModule::new(None);
    ast.crate_attributes.main_setting = CrateMainSetting::NoMain { span: None };
    let mut mir = module_with_functions(Vec::new());
    mir.attributes.no_main = true;
    let target = Target::default();
    let mut options = CodegenOptions::default();
    options.backend = Backend::Wasm;
    let perf = PerfMetadata::default();
    let output = std::env::temp_dir().join(format!("chic-test-no-main-{}", std::process::id()));
    let result = super::compile(
        &ast,
        &mir,
        &perf,
        &target,
        ChicKind::Executable,
        &output,
        &options,
        &[],
        &[],
        &[],
        &[],
    );
    assert!(
        result.is_ok(),
        "compile should allow no_main executable without Main, got {result:?}"
    );
    if output.exists() {
        let _ = std::fs::remove_file(&output);
    }
}

#[test]
fn find_entry_function_returns_none_when_missing() {
    let ast = ast_module_without_main();
    assert!(super::find_entry_function(&ast).is_none());
}

#[test]
fn qualify_joins_namespace_correctly() {
    assert_eq!(super::qualify(None, "Main"), "Main");
    assert_eq!(super::qualify(Some("Outer"), "Main"), "Outer::Main");
}

#[test]
fn compile_emits_wasm_file_for_valid_module() {
    let ast = simple_ast_module_with_main();
    let mir = module_with_functions(vec![simple_function(
        "Main",
        FunctionKind::Function,
        Ty::named("int"),
    )]);
    let target = Target::default();
    let mut options = CodegenOptions::default();
    options.backend = Backend::Wasm;
    let perf = PerfMetadata::default();
    let output =
        std::env::temp_dir().join(format!("chic-test-compile-success-{}", std::process::id()));

    let artifact = super::compile(
        &ast,
        &mir,
        &perf,
        &target,
        ChicKind::Executable,
        &output,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("compile should succeed with main function");
    assert!(output.exists(), "wasm output file should exist");
    assert!(
        artifact.artifact_path.exists(),
        "artifact path should be present"
    );
    let bytes = fs::read(&output).expect("read emitted wasm bytes");
    assert!(
        bytes.starts_with(&WASM_MAGIC),
        "emitted bytes should begin with wasm magic"
    );
    let _ = fs::remove_file(&output);
}

#[test]
fn compile_emits_wat_text_when_requested() {
    let ast = simple_ast_module_with_main();
    let mir = module_with_functions(vec![simple_function(
        "Main",
        FunctionKind::Function,
        Ty::named("int"),
    )]);
    let target = Target::default();
    let mut options = CodegenOptions::default();
    options.backend = Backend::Wasm;
    options.emit_wat_text = true;
    let perf = PerfMetadata::default();
    let output = std::env::temp_dir().join(format!("chic-test-emit-wat-{}", std::process::id()));

    super::compile(
        &ast,
        &mir,
        &perf,
        &target,
        ChicKind::Executable,
        &output,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("compile should succeed with textual emission enabled");

    let wat_path = output.with_extension("wat");
    assert!(wat_path.exists(), ".wat companion artifact should exist");
    let contents = fs::read_to_string(&wat_path).expect("read generated .wat file");
    assert!(
        contents.contains("(module"),
        "textual module header missing: {contents}"
    );
    assert!(
        contents.contains("(func $Main (type"),
        "expected sanitized function label in textual output: {contents}"
    );
    assert!(
        contents.contains("(import \"chic_rt\" \"panic\""),
        "runtime panic hook should be described in textual output: {contents}"
    );

    let _ = fs::remove_file(&output);
    let _ = fs::remove_file(&wat_path);
}

#[test]
fn find_entry_function_discovers_nested_main() {
    let ast = nested_namespace_ast_module();
    let entry = super::find_entry_function(&ast).expect("expected nested entry");
    assert_eq!(entry, "Outer::Inner::Main");
}

#[test]
fn compile_rejects_vectors_until_wasm_backend_supports_them() {
    let ast = simple_ast_module_with_main();
    let vector_ty = Ty::Vector(VectorTy {
        element: Box::new(Ty::named("int")),
        lanes: 4,
    });
    let mir = module_with_functions(vec![simple_function(
        "Main",
        FunctionKind::Function,
        vector_ty,
    )]);
    let target = Target::default();
    let mut options = CodegenOptions::default();
    options.backend = Backend::Wasm;
    let perf = PerfMetadata::default();
    let output =
        std::env::temp_dir().join(format!("chic-test-vector-rejection-{}", std::process::id()));

    let err = super::compile(
        &ast,
        &mir,
        &perf,
        &target,
        ChicKind::Executable,
        &output,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect_err("compile should reject vector types on wasm backend");
    assert!(
        err.to_string().contains("SIMD vectors"),
        "expected SIMD rejection diagnostic, got: {err}"
    );
    if output.exists() {
        let _ = std::fs::remove_file(&output);
    }
}

#[test]
fn module_contains_vectors_detects_vector_usage() {
    let vector_ty = Ty::Vector(VectorTy {
        element: Box::new(Ty::named("float")),
        lanes: 4,
    });
    let mut mir_with_vector = module_with_functions(vec![simple_function(
        "Main",
        FunctionKind::Function,
        Ty::named("int"),
    )]);
    if let Some(main) = mir_with_vector.functions.first_mut() {
        main.body.locals.push(LocalDecl::new(
            Some("vec".into()),
            vector_ty,
            false,
            None,
            LocalKind::Local,
        ));
    }
    assert!(
        super::module_contains_vectors(&mir_with_vector),
        "vector locals should be detected"
    );

    let mir_without_vector = module_with_functions(vec![simple_function(
        "Main",
        FunctionKind::Function,
        Ty::named("int"),
    )]);
    assert!(
        !super::module_contains_vectors(&mir_without_vector),
        "modules without vectors should not trigger the gate"
    );
}

#[test]
fn compile_rejects_non_wasm_backend() {
    let ast = simple_ast_module_with_main();
    let mir = module_with_functions(vec![simple_function(
        "Main",
        FunctionKind::Function,
        Ty::named("int"),
    )]);
    let target = Target::default();
    let mut options = CodegenOptions::default();
    options.backend = Backend::Llvm;
    let perf = PerfMetadata::default();
    let output =
        std::env::temp_dir().join(format!("chic-test-non-wasm-backend-{}", std::process::id()));

    let err = super::compile(
        &ast,
        &mir,
        &perf,
        &target,
        ChicKind::Executable,
        &output,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect_err("compile should reject non-WASM backend options");
    assert!(
        format!("{err}").contains("non-WASM options state"),
        "unexpected error message for backend mismatch: {err}"
    );
    if output.exists() {
        let _ = std::fs::remove_file(&output);
    }
}

#[test]
fn find_entry_function_prefers_root_main() {
    let mut module = simple_ast_module_with_main();
    let mut nested = NamespaceDecl {
        name: "Nested".into(),
        items: Vec::new(),
        doc: None,
        attributes: Vec::new(),
        span: None,
    };
    nested.items.push(AstItem::Function(FunctionDecl {
        visibility: Visibility::Public,
        name: "Main".into(),
        name_span: None,
        signature: Signature {
            parameters: Vec::new(),
            return_type: TypeExpr::simple("int"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(AstBlock {
            statements: Vec::new(),
            span: None,
        }),
        is_async: false,
        is_constexpr: false,
        doc: None,
        modifiers: Vec::new(),
        is_unsafe: false,
        attributes: Vec::new(),
        is_extern: false,
        extern_abi: None,
        extern_options: None,
        link_name: None,
        link_library: None,
        operator: None,
        generics: None,
        vectorize_hint: None,
        dispatch: MemberDispatch::default(),
    }));
    module.push_item(AstItem::Namespace(nested));

    let entry = super::find_entry_function(&module).expect("expected root entry");
    assert_eq!(entry, "Main", "root-level Main should take precedence");
}
