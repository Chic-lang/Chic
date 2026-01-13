#![cfg(test)]

use super::super::common::*;
use crate::chic_kind::ChicKind;
use crate::mir::*;

#[test]
fn module_builder_export_section_exports_all_for_static_library() {
    let functions = vec![
        simple_function("Main", FunctionKind::Function, Ty::Unit),
        simple_function("Helper", FunctionKind::Function, Ty::Unit),
    ];
    let harness = WasmFunctionHarness::from_module(module_with_functions(functions));
    let builder = harness
        .module_builder(Some("Main"), ChicKind::StaticLibrary)
        .expect("construct module builder");
    let exports = builder
        .emit_export_section()
        .expect("export section result")
        .expect("static library should export functions");
    let payload = exports.payload_bytes();
    let names = decode_exports(payload)
        .into_iter()
        .map(|(name, _, _)| name)
        .collect::<Vec<_>>();
    assert!(names.contains(&"chic_main".to_string()));
    assert!(names.contains(&"Main".to_string()));
    assert!(names.contains(&"Helper".to_string()));
}

#[test]
fn module_builder_export_section_exports_tests_for_dynamic_library() {
    let testcase = simple_function("Check", FunctionKind::Testcase, Ty::Unit);
    let helper = simple_function("Helper", FunctionKind::Function, Ty::Unit);
    let harness = WasmFunctionHarness::from_module(module_with_functions(vec![testcase, helper]));
    let builder = harness
        .module_builder(None, ChicKind::DynamicLibrary)
        .expect("construct module builder");
    let exports = builder
        .emit_export_section()
        .expect("export section result")
        .expect("dynamic library should export symbols");
    let payload = exports.payload_bytes();
    let names = decode_exports(payload)
        .into_iter()
        .map(|(name, _, _)| name)
        .collect::<Vec<_>>();
    assert!(names.contains(&"test::Check".to_string()));
    assert!(names.contains(&"Helper".to_string()));
}

#[test]
fn module_builder_export_section_covers_entry_and_tests() {
    let entry = simple_function("Main", FunctionKind::Function, Ty::Unit);
    let helper = simple_function("Helper", FunctionKind::Function, Ty::Unit);
    let testcase = simple_function("Check", FunctionKind::Testcase, Ty::Unit);
    let harness =
        WasmFunctionHarness::from_module(module_with_functions(vec![entry, helper, testcase]));
    let builder = harness
        .module_builder(Some("Main"), ChicKind::Executable)
        .unwrap();

    let section = builder
        .emit_export_section()
        .expect("export section result")
        .expect("exports expected for executable");
    let exports = decode_exports(section.payload_bytes());
    let names: Vec<_> = exports.iter().map(|(name, _, _)| name.clone()).collect();
    assert!(
        names.contains(&"chic_main".to_string()),
        "entry wrapper should export chic_main"
    );
    assert!(
        names.contains(&"test::Check".to_string()),
        "testcase export expected"
    );
    assert!(
        !names.contains(&"Helper".to_string()),
        "non-entry helper should not be exported for executables"
    );
}

#[test]
fn module_builder_export_section_returns_none_for_static_library_without_functions() {
    let harness = WasmFunctionHarness::from_module(module_with_functions(Vec::new()));
    let builder = harness
        .module_builder(None, ChicKind::StaticLibrary)
        .expect("construct module builder");
    assert!(
        builder
            .emit_export_section()
            .expect("export section result")
            .is_none(),
        "empty static libraries should not emit export sections"
    );
}

#[test]
fn module_builder_export_section_includes_all_functions_for_library() {
    let entry = simple_function("Main", FunctionKind::Function, Ty::Unit);
    let helper = simple_function("Helper", FunctionKind::Function, Ty::Unit);
    let harness = WasmFunctionHarness::from_module(module_with_functions(vec![entry, helper]));
    let builder = harness
        .module_builder(Some("Main"), ChicKind::StaticLibrary)
        .unwrap();

    let section = builder
        .emit_export_section()
        .expect("export section result")
        .expect("static library should export entries");
    let exports = decode_exports(section.payload_bytes());
    let names: Vec<_> = exports.iter().map(|(name, _, _)| name.clone()).collect();
    assert!(names.contains(&"chic_main".to_string()));
    assert!(names.contains(&"Main".to_string()));
    assert!(names.contains(&"Helper".to_string()));
}

#[test]
fn module_builder_export_section_returns_none_when_no_exports() {
    let harness = WasmFunctionHarness::from_module(module_with_functions(vec![simple_function(
        "Utility",
        FunctionKind::Function,
        Ty::Unit,
    )]));
    let builder = harness
        .module_builder(None, ChicKind::Executable)
        .expect("builder");
    assert!(
        builder
            .emit_export_section()
            .expect("export section")
            .is_none(),
        "expected no exports when no entry or testcase"
    );
}
