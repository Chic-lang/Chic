use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Instant;

use crate::error::Result;
use crate::manifest::Manifest;
use crate::mir::async_types::task_result_ty;
use crate::mir::{GenericArg, MirFunction, MirModule, Ty, TypeLayout, TypeLayoutTable};
use crate::runtime::wasm_executor::AsyncLayoutOverrides;
use crate::runtime::{
    WasmExecutionOptions, WasmExecutionTrace, WasmProgram, WasmProgramExportOutcome, WasmValue,
};
use crate::target::Target;
use tracing::info;

use super::{FrontendReport, TestCaseResult, TestStatus};
use crate::driver::types::TestOptions;
use crate::mir::TestCaseMetadata;

const WASM_TEST_THREAD_STACK_SIZE: usize = 128 * 1024 * 1024;

pub(crate) fn resolve_wasm_options(path: &Path, target: &Target) -> Result<WasmExecutionOptions> {
    let manifest = match Manifest::discover(path)? {
        Some(manifest) => manifest,
        None => return Ok(WasmExecutionOptions::default()),
    };
    let settings = match manifest.wasm_settings_for_target(target) {
        Some(settings) => settings,
        None => return Ok(WasmExecutionOptions::default()),
    };
    Ok(settings.to_execution_options())
}

pub(crate) fn collect_wasm_testcases(
    report: &FrontendReport,
    program_bytes: &[u8],
    options: &WasmExecutionOptions,
    trace_enabled: bool,
    test_options: &TestOptions,
    _target: &Target,
    allowed_roots: Option<&[PathBuf]>,
) -> (Vec<TestCaseResult>, usize) {
    if trace_enabled {
        info!(target: "pipeline", stage = "wasm.collect_testcases.start", functions = report.mir_module.functions.len());
    }
    if std::env::var_os("CHIC_DEBUG_WASM_DUMP").is_some() {
        let dump_path = PathBuf::from("obj/last_test.wasm");
        let _ = std::fs::create_dir_all("obj");
        if let Err(err) = std::fs::write(&dump_path, program_bytes) {
            eprintln!("[wasm-dump] failed to write {}: {err}", dump_path.display());
        } else {
            eprintln!("[wasm-dump] wrote {}", dump_path.display());
        }
    }
    let mut all_metadata = crate::mir::collect_test_metadata(&report.mir_module);
    all_metadata.sort_by(|a, b| a.id.cmp(&b.id));
    let selected: Vec<_> = all_metadata
        .iter()
        .filter(|meta| {
            if !test_options.selection.matches(meta) || should_skip_for_executor(meta, true) {
                return false;
            }
            let Some(roots) = allowed_roots else {
                return true;
            };
            let Some(span) = meta.span else {
                return true;
            };
            let Some(path) = report.files.path(span.file_id) else {
                return true;
            };
            let cwd = std::env::current_dir().ok();
            let to_abs = |p: &Path| -> PathBuf {
                if p.is_absolute() {
                    p.to_path_buf()
                } else if let Some(cwd) = &cwd {
                    cwd.join(p)
                } else {
                    p.to_path_buf()
                }
            };
            let abs_path = to_abs(path);
            roots.iter().any(|root| abs_path.starts_with(to_abs(root)))
        })
        .cloned()
        .collect();
    let filtered_out = all_metadata.len().saturating_sub(selected.len());
    if selected.is_empty() {
        return (Vec::new(), filtered_out);
    }

    let bytes = Arc::new(program_bytes.to_vec());
    let layouts = Arc::new(report.mir_module.type_layouts.clone());
    let functions: Vec<MirFunction> = report.mir_module.functions.clone();
    let watchdog = test_options.watchdog;
    let parallelism = if options.coverage_hook.is_some() {
        1
    } else {
        test_options
            .parallelism
            .or_else(|| {
                std::thread::available_parallelism()
                    .map(|val| val.get())
                    .ok()
            })
            .unwrap_or(1)
            .max(1)
    };

    let mut cases: Vec<TestCaseResult> = Vec::new();
    let mut slots: Vec<Option<TestCaseResult>> = std::iter::repeat_with(|| None)
        .take(selected.len())
        .collect();
    let mut scheduled: Vec<Option<(TestCaseMetadata, MirFunction)>> =
        std::iter::repeat_with(|| None)
            .take(selected.len())
            .collect();
    for (index, meta) in selected.into_iter().enumerate() {
        if !meta.parameters.is_empty() {
            slots[index] = Some(skip_parameterized(&meta));
            continue;
        }
        match functions.get(meta.function_index).cloned() {
            Some(function) => scheduled[index] = Some((meta, function)),
            None => {
                slots[index] = Some(missing_function_result(&meta));
            }
        }
    }

    if test_options.fail_fast || parallelism == 1 {
        let fail_fast = test_options.fail_fast;
        let options = options.clone();
        let (tx, rx) = mpsc::channel();
        let _ = thread::Builder::new()
            .name("wasm-test-worker".into())
            .stack_size(WASM_TEST_THREAD_STACK_SIZE)
            .spawn(move || {
                let mut cases: Vec<TestCaseResult> = Vec::new();
                if fail_fast {
                    let mut saw_failure = false;
                    for index in 0..slots.len() {
                        if let Some(result) = slots[index].take() {
                            cases.push(result);
                            continue;
                        }
                        let Some((meta, function)) = scheduled[index].take() else {
                            continue;
                        };
                        if saw_failure {
                            cases.push(skip_after_fail_fast(&meta));
                            continue;
                        }
                        let result = execute_wasm_case(
                            &meta,
                            function,
                            bytes.clone(),
                            layouts.clone(),
                            options.clone(),
                            watchdog,
                            trace_enabled,
                        );
                        if matches!(result.status, TestStatus::Failed) {
                            saw_failure = true;
                        }
                        cases.push(result);
                    }
                } else {
                    for index in 0..slots.len() {
                        if let Some(result) = slots[index].take() {
                            cases.push(result);
                            continue;
                        }
                        if let Some((meta, function)) = scheduled[index].take() {
                            cases.push(execute_wasm_case(
                                &meta,
                                function,
                                bytes.clone(),
                                layouts.clone(),
                                options.clone(),
                                watchdog,
                                trace_enabled,
                            ));
                        }
                    }
                }
                let _ = tx.send(cases);
            });
        let cases = rx.recv().unwrap_or_default();
        return (cases, filtered_out);
    }

    let mut tasks: Vec<(usize, TestCaseMetadata, MirFunction)> = Vec::new();
    for (index, entry) in scheduled.into_iter().enumerate() {
        if let Some((meta, function)) = entry {
            tasks.push((index, meta, function));
        }
    }
    let queue = Arc::new(Mutex::new(tasks));
    let (tx, rx) = mpsc::channel();
    for _ in 0..parallelism {
        let tx = tx.clone();
        let queue = queue.clone();
        let bytes = bytes.clone();
        let layouts = layouts.clone();
        let options = options.clone();
        let _ = thread::Builder::new()
            .name("wasm-test-worker".into())
            .stack_size(WASM_TEST_THREAD_STACK_SIZE)
            .spawn(move || {
                loop {
                    let task = {
                        let mut guard = queue.lock().unwrap();
                        guard.pop()
                    };
                    let Some((index, meta, function)) = task else {
                        break;
                    };
                    let result = execute_wasm_case(
                        &meta,
                        function,
                        bytes.clone(),
                        layouts.clone(),
                        options.clone(),
                        watchdog,
                        false,
                    );
                    let _ = tx.send((index, result));
                }
            });
    }
    drop(tx);

    for (index, result) in rx {
        if let Some(slot) = slots.get_mut(index) {
            *slot = Some(result);
        }
    }

    for result in slots.into_iter().flatten() {
        cases.push(result);
    }

    (cases, filtered_out)
}

fn should_skip_for_executor(meta: &TestCaseMetadata, is_wasm_executor: bool) -> bool {
    if is_wasm_executor {
        return meta
            .categories
            .iter()
            .any(|category| category.eq_ignore_ascii_case("native"));
    }
    meta.categories
        .iter()
        .any(|category| category.eq_ignore_ascii_case("wasm"))
}

fn skip_after_fail_fast(meta: &TestCaseMetadata) -> TestCaseResult {
    TestCaseResult {
        id: meta.id.clone(),
        name: meta.name.clone(),
        qualified_name: meta.qualified_name.clone(),
        namespace: meta.namespace.clone(),
        categories: meta.categories.clone(),
        is_async: meta.is_async,
        status: TestStatus::Skipped,
        message: Some("skipped due to --fail-fast".into()),
        wasm_trace: None,
        duration: None,
    }
}

fn missing_function_result(meta: &TestCaseMetadata) -> TestCaseResult {
    TestCaseResult {
        id: meta.id.clone(),
        name: meta.name.clone(),
        qualified_name: meta.qualified_name.clone(),
        namespace: meta.namespace.clone(),
        categories: meta.categories.clone(),
        is_async: meta.is_async,
        status: TestStatus::Failed,
        message: Some(format!(
            "test metadata refers to missing function index {}",
            meta.function_index
        )),
        wasm_trace: None,
        duration: None,
    }
}

fn skip_parameterized(meta: &TestCaseMetadata) -> TestCaseResult {
    TestCaseResult {
        id: meta.id.clone(),
        name: meta.name.clone(),
        qualified_name: meta.qualified_name.clone(),
        namespace: meta.namespace.clone(),
        categories: meta.categories.clone(),
        is_async: meta.is_async,
        status: TestStatus::Skipped,
        message: Some("parameterized testcases are not supported yet".into()),
        wasm_trace: None,
        duration: None,
    }
}

fn execute_wasm_case(
    meta: &TestCaseMetadata,
    function: MirFunction,
    bytes: Arc<Vec<u8>>,
    layouts: Arc<TypeLayoutTable>,
    options: WasmExecutionOptions,
    watchdog: crate::driver::types::WatchdogConfig,
    trace_enabled: bool,
) -> TestCaseResult {
    if std::env::var_os("CHIC_DEBUG_TESTCASE_RET").is_some() {
        eprintln!(
            "[testcase-ret] meta={} idx={} func={} -> {:?}",
            meta.qualified_name, meta.function_index, function.name, function.signature.ret
        );
    }
    let export_candidates = [
        format!("test::{}", meta.qualified_name),
        meta.qualified_name.clone(),
    ];
    let start = Instant::now();
    let (status, message, trace) = if let Some(timeout) = watchdog.timeout {
        let (tx, rx) = mpsc::channel();
        let bytes_clone = bytes.clone();
        let layouts = layouts.clone();
        let options_for_thread = options.clone();
        let options_for_timeout = options.clone();
        let function = function.clone();
        let _ = thread::Builder::new()
            .name("wasm-test-watchdog".into())
            .stack_size(WASM_TEST_THREAD_STACK_SIZE)
            .spawn(move || {
                let program = match WasmProgram::from_bytes(&bytes_clone) {
                    Ok(program) => program,
                    Err(err) => {
                        let _ = tx.send((
                            TestStatus::Failed,
                            Some(err.message),
                            WasmExecutionTrace::from_options(&options_for_thread),
                        ));
                        return;
                    }
                };
                let result = wasm_case_status(
                    &layouts,
                    &program,
                    &function,
                    &export_candidates,
                    &options_for_thread,
                );
                let _ = tx.send(result);
            });
        match rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(_) => (
                TestStatus::Failed,
                Some(format!("watchdog timeout after {}ms", timeout.as_millis())),
                WasmExecutionTrace::from_options(&options_for_timeout),
            ),
        }
    } else {
        let program = match WasmProgram::from_bytes(&bytes) {
            Ok(program) => program,
            Err(err) => {
                return TestCaseResult {
                    id: meta.id.clone(),
                    name: meta.name.clone(),
                    qualified_name: meta.qualified_name.clone(),
                    namespace: meta.namespace.clone(),
                    categories: meta.categories.clone(),
                    is_async: meta.is_async,
                    status: TestStatus::Failed,
                    message: Some(err.message),
                    wasm_trace: Some(WasmExecutionTrace::from_options(&options)),
                    duration: Some(start.elapsed()),
                };
            }
        };
        wasm_case_status(&layouts, &program, &function, &export_candidates, &options)
    };

    if trace_enabled {
        info!(
            target: "pipeline",
            stage = "wasm.collect_testcases.case",
            testcase = meta.qualified_name.as_str(),
            status = ?status
        );
    }

    TestCaseResult {
        id: meta.id.clone(),
        name: meta.name.clone(),
        qualified_name: meta.qualified_name.clone(),
        namespace: meta.namespace.clone(),
        categories: meta.categories.clone(),
        is_async: meta.is_async,
        status,
        message,
        wasm_trace: Some(trace),
        duration: Some(start.elapsed()),
    }
}

pub(crate) fn wasm_case_status(
    layouts: &TypeLayoutTable,
    program: &WasmProgram,
    function: &MirFunction,
    export_candidates: &[String; 2],
    options: &WasmExecutionOptions,
) -> (TestStatus, Option<String>, WasmExecutionTrace) {
    for export in export_candidates {
        if !program.has_export(export) {
            continue;
        }
        if std::env::var_os("CHIC_DEBUG_WASM_EXPORT_INDEX").is_some() {
            if let Some(index) = program.export_index(export) {
                eprintln!("[wasm-export] {} -> {}", export, index);
            }
        }
        let mut case_options = options.clone();
        if let Some(overrides) = async_layout_overrides(&function.signature.ret, layouts) {
            case_options.async_layout = Some(overrides);
        }
        if let Some((len, align)) = expected_async_result_layout(&function.signature.ret, layouts) {
            case_options.async_result_len = Some(len);
            case_options.async_result_align = Some(align);
        }
        match program.execute_export_with_options(export, &[], &case_options) {
            Ok(outcome) => {
                let WasmProgramExportOutcome { value, trace } = outcome;
                let (status, message) =
                    evaluate_wasm_testcase_result(&function.signature.ret, value);
                return (status, message, trace);
            }
            Err(err) => {
                return (
                    TestStatus::Failed,
                    Some(err.message),
                    WasmExecutionTrace::from_options(&case_options),
                );
            }
        }
    }

    (
        TestStatus::Failed,
        Some("testcase is not exported in wasm artifact".into()),
        WasmExecutionTrace::from_options(options),
    )
}

pub(crate) fn find_entry_function(module: &MirModule) -> Option<&str> {
    module
        .functions
        .iter()
        .find(|func| {
            matches!(func.kind, crate::mir::FunctionKind::Function)
                && !func.is_local()
                && func.name.rsplit("::").next() == Some("Main")
        })
        .map(|func| func.name.as_str())
}

fn layout_field_offset(layouts: &TypeLayoutTable, ty: &Ty, field: &str) -> Option<usize> {
    let layout = layouts.layout_for_name(&ty.canonical_name())?;
    let data = match layout {
        TypeLayout::Struct(data) | TypeLayout::Class(data) => data,
        _ => return None,
    };
    data.fields.iter().find(|f| f.name == field)?.offset
}

fn async_layout_overrides(ret_ty: &Ty, layouts: &TypeLayoutTable) -> Option<AsyncLayoutOverrides> {
    let inner = task_result_ty(ret_ty)?;
    let task_ty = Ty::named_generic("Std::Async::Task", vec![GenericArg::Type(inner.clone())]);
    let future_ty = Ty::named_generic("Std::Async::Future", vec![GenericArg::Type(inner.clone())]);
    let header_ty = Ty::named("Std::Async::FutureHeader");

    let header_offset = layout_field_offset(layouts, &future_ty, "Header")?;
    let header_state_offset = layout_field_offset(layouts, &header_ty, "StatePointer")?;
    let header_vtable_offset = layout_field_offset(layouts, &header_ty, "VTablePointer")?;
    let header_exec_ctx_offset = layout_field_offset(layouts, &header_ty, "ExecutorContext")?;
    let header_flags_offset = layout_field_offset(layouts, &header_ty, "Flags")?;
    let future_completed_offset = layout_field_offset(layouts, &future_ty, "Completed")?;
    let future_result_offset = layout_field_offset(layouts, &future_ty, "Result")?;
    let task_flags_offset = layout_field_offset(layouts, &task_ty, "Flags")?;
    let task_inner_future_offset = layout_field_offset(layouts, &task_ty, "InnerFuture")?;

    if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
        if let Some(TypeLayout::Struct(future_layout) | TypeLayout::Class(future_layout)) =
            layouts.layout_for_name(&future_ty.canonical_name())
        {
            let mut fields: Vec<(String, Option<usize>)> = future_layout
                .fields
                .iter()
                .map(|f| (f.name.clone(), f.offset))
                .collect();
            fields.sort_by(|a, b| a.0.cmp(&b.0));
            eprintln!(
                "[wasm-async] future layout {} fields={:?}",
                future_ty.canonical_name(),
                fields
            );
        }
        if let Some(TypeLayout::Struct(task_layout) | TypeLayout::Class(task_layout)) =
            layouts.layout_for_name(&task_ty.canonical_name())
        {
            let mut fields: Vec<(String, Option<usize>)> = task_layout
                .fields
                .iter()
                .map(|f| (f.name.clone(), f.offset))
                .collect();
            fields.sort_by(|a, b| a.0.cmp(&b.0));
            eprintln!(
                "[wasm-async] task layout {} fields={:?}",
                task_ty.canonical_name(),
                fields
            );
        }
    }

    let to_u32 = |value: usize| u32::try_from(value).ok();
    let overrides = AsyncLayoutOverrides {
        future_header_state_offset: to_u32(header_offset.checked_add(header_state_offset)?),
        future_header_vtable_offset: to_u32(header_offset.checked_add(header_vtable_offset)?),
        future_header_executor_context_offset: to_u32(
            header_offset.checked_add(header_exec_ctx_offset)?,
        ),
        future_header_flags_offset: to_u32(header_offset.checked_add(header_flags_offset)?),
        future_completed_offset: to_u32(future_completed_offset),
        future_result_offset: to_u32(future_result_offset),
        task_flags_offset: to_u32(task_flags_offset),
        task_inner_future_offset: to_u32(task_inner_future_offset),
    };

    if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
        eprintln!(
            "[wasm-async] async layout overrides for {}: header_state={} header_vtable={} header_ctx={} header_flags={} completed={} result={} task_flags={} task_inner={}",
            ret_ty.canonical_name(),
            overrides.future_header_state_offset.unwrap_or_default(),
            overrides.future_header_vtable_offset.unwrap_or_default(),
            overrides
                .future_header_executor_context_offset
                .unwrap_or_default(),
            overrides.future_header_flags_offset.unwrap_or_default(),
            overrides.future_completed_offset.unwrap_or_default(),
            overrides.future_result_offset.unwrap_or_default(),
            overrides.task_flags_offset.unwrap_or_default(),
            overrides.task_inner_future_offset.unwrap_or_default(),
        );
    }

    Some(overrides)
}

fn expected_async_result_layout(ret_ty: &Ty, layouts: &TypeLayoutTable) -> Option<(u32, u32)> {
    let mut ty = ret_ty.clone();
    if let Ty::Named(named) = &ty {
        let base_name = named.name.as_str();
        let short = base_name.rsplit("::").next().unwrap_or(base_name);
        if short.eq_ignore_ascii_case("task") || base_name.eq_ignore_ascii_case("std::async::task")
        {
            if let Some(inner) = named.nth_type_arg(0) {
                ty = inner.clone();
            }
        }
    }
    layouts
        .size_and_align_for_ty(&ty)
        .and_then(|(size, align)| {
            u32::try_from(size)
                .ok()
                .and_then(|len| u32::try_from(align).ok().map(|a| (len, a)))
        })
        .or_else(|| {
            if let Ty::Named(named) = &ty {
                let lowered = named.name.to_ascii_lowercase();
                if lowered == "bool" || lowered == "system.boolean" {
                    return Some((1, 1));
                }
            }
            None
        })
}

fn evaluate_wasm_testcase_result(
    ret_ty: &Ty,
    result: Option<WasmValue>,
) -> (TestStatus, Option<String>) {
    if let Ty::Named(named) = ret_ty {
        let base_name = named.name.as_str();
        let short = base_name.rsplit("::").next().unwrap_or(base_name);
        if short.eq_ignore_ascii_case("task") || base_name.eq_ignore_ascii_case("std::async::task")
        {
            let inner = named.nth_type_arg(0).cloned().unwrap_or_else(|| Ty::Unit);
            return evaluate_wasm_testcase_result(&inner, result);
        }
    }
    match ret_ty {
        Ty::Unit | Ty::Unknown => (TestStatus::Passed, None),
        Ty::Vector(_)
        | Ty::Array(_)
        | Ty::Vec(_)
        | Ty::Span(_)
        | Ty::ReadOnlySpan(_)
        | Ty::String
        | Ty::Str
        | Ty::Tuple(_) | Ty::Fn(_) | Ty::Pointer(_) | Ty::Ref(_) | Ty::Rc(_) | Ty::Arc(_)
        | Ty::TraitObject(_) => (
            TestStatus::Failed,
            Some(
                "collection, string, or tuple return types are not yet supported by the test runner"
                    .into(),
            ),
        ),
        Ty::Nullable(_) => (
            TestStatus::Failed,
            Some("nullable return types are not yet supported by the test runner".into()),
        ),
        Ty::Named(name) => {
            let lowered = name.to_ascii_lowercase();
            match lowered.as_str() {
                "void" | "system.void" => (TestStatus::Passed, None),
                "bool" | "system.boolean" => match result {
                    Some(value) => {
                        if value.as_bool().unwrap_or(false) {
                            (TestStatus::Passed, None)
                        } else {
                            (TestStatus::Failed, Some("test returned false".into()))
                        }
                    }
                    None => (
                        TestStatus::Failed,
                        Some("test did not return a value".into()),
                    ),
                },
                "int" | "i32" | "uint" | "u32" | "system.int32" => match result {
                    Some(value) => {
                        let int_value = value.as_i32().unwrap_or(0);
                        if int_value == 0 {
                            (TestStatus::Passed, None)
                        } else {
                            (
                                TestStatus::Failed,
                                Some(format!("test returned {int_value}")),
                            )
                        }
                    }
                    None => (
                        TestStatus::Failed,
                        Some("test did not return a value".into()),
                    ),
                },
                "long" | "i64" | "ulong" | "u64" | "system.int64" => match result {
                    Some(value) => {
                        let long_value = value.as_i64().unwrap_or(0);
                        if long_value == 0 {
                            (TestStatus::Passed, None)
                        } else {
                            (
                                TestStatus::Failed,
                                Some(format!("test returned {long_value}")),
                            )
                        }
                    }
                    None => (
                        TestStatus::Failed,
                        Some("test did not return a value".into()),
                    ),
                },
                _ => match result {
                    Some(value) => {
                        if value.as_bool().unwrap_or(false) {
                            (TestStatus::Passed, None)
                        } else {
                            (
                                TestStatus::Failed,
                                Some(format!("unsupported return type {name}: {value:?}")),
                            )
                        }
                    }
                    None => (
                        TestStatus::Failed,
                        Some(format!("test of type {name} did not return a value")),
                    ),
                },
            }
        }
    }
}
