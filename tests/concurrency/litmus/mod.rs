use chic::codegen::{Backend, CpuIsaConfig};
use chic::driver::{BuildFfiOptions, BuildRequest, CompilerDriver, TestOptions, TestStatus};
use chic::logging::LogLevel;
use chic::manifest::MissingDocsRule;
use chic::runtime::backend::RuntimeBackend;
use chic::{ChicKind, Target};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::Builder;

fn litmus_enabled() -> bool {
    matches!(
        std::env::var("CHIC_ENABLE_LITMUS"),
        Ok(value) if value == "1" || value.eq_ignore_ascii_case("true")
    )
}

fn litmus_sources() -> Vec<PathBuf> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("concurrency")
        .join("litmus");
    let all = [
        "fixtures.cl",
        "store_buffering.cl",
        "load_buffering.cl",
        "iriw.cl",
        "message_passing.cl",
    ];
    let filtered = if let Ok(filter) = std::env::var("CHIC_LITMUS_FILTER") {
        let mut allowed = std::collections::HashSet::new();
        for part in filter.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                allowed.insert(trimmed);
            }
        }
        all.iter()
            .filter(|file| allowed.contains(**file))
            .map(|file| base.join(file))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    if filtered.is_empty() {
        all.iter().map(|file| base.join(file)).collect()
    } else {
        filtered
    }
}

fn run_suite(backend: Backend) -> Result<(), Box<dyn Error>> {
    let driver = CompilerDriver::new();
    let sources = litmus_sources();
    eprintln!("running {backend:?} litmus sources: {sources:?}");

    match backend {
        Backend::Llvm => run_suite_native(&driver, &sources)?,
        Backend::Wasm => run_suite_interpreter(&driver, Backend::Wasm, &sources)?,
        other => return Err(format!("litmus suite does not support backend {:?}", other).into()),
    }

    Ok(())
}

fn run_suite_interpreter(
    driver: &CompilerDriver,
    backend: Backend,
    sources: &[PathBuf],
) -> Result<(), Box<dyn Error>> {
    let request = BuildRequest {
        inputs: sources.to_vec(),
        manifest: None,
        workspace: None,
        target: Target::host(),
        kind: ChicKind::StaticLibrary,
        backend,
        runtime_backend: RuntimeBackend::Chic,
        output: None,
        run_timeout: None,
        emit_wat_text: false,
        emit_object: false,
        coverage: false,
        cpu_isa: CpuIsaConfig::baseline(),
        emit_header: false,
        emit_library_pack: false,
        cc1_args: Vec::new(),
        cc1_keep_temps: false,
        load_stdlib: true,
        trace_pipeline: false,
        trait_solver_metrics: false,
        defines: Vec::new(),
        log_level: LogLevel::Info,
        ffi: BuildFfiOptions::default(),
        configuration: "Debug".to_string(),
        framework: None,
        artifacts_path: None,
        obj_dir: None,
        bin_dir: None,
        no_dependencies: false,
        no_restore: false,
        no_incremental: false,
        rebuild: false,
        incremental_validate: false,
        clean_only: false,
        disable_build_servers: false,
        source_root: None,
        properties: Vec::new(),
        verbosity: chic::driver::types::Verbosity::Normal,
        telemetry: chic::driver::types::TelemetrySetting::Auto,
        version_suffix: None,
        nologo: false,
        force: false,
        interactive: false,
        self_contained: None,
        doc_enforcement: MissingDocsRule::default(),
    };
    let run = driver.run_tests(request, TestOptions::default())?;
    if let Some(artifact) = run.report.artifact.as_ref()
        && let Some(dest) = std::env::var_os("CHIC_DEBUG_LITMUS_WASM")
    {
        eprintln!("wasm artifact for litmus run: {}", artifact.display());
        if !dest.is_empty() {
            let dest_path = PathBuf::from(dest);
            std::fs::copy(artifact, &dest_path)?;
            eprintln!("copied litmus wasm to {}", dest_path.display());
        }
    }

    let mut failures = Vec::new();
    for case in &run.cases {
        if case.status != TestStatus::Passed {
            let detail = case
                .message
                .as_deref()
                .unwrap_or("<no diagnostic provided>");
            let trace = case.wasm_trace.as_ref();
            let mut trace_fragments = Vec::new();
            if let Some(trace) = trace {
                if !trace.stdout.is_empty() {
                    trace_fragments.push(format!(
                        "stdout=`{}`",
                        String::from_utf8_lossy(&trace.stdout)
                    ));
                }
                if !trace.stderr.is_empty() {
                    trace_fragments.push(format!(
                        "stderr=`{}`",
                        String::from_utf8_lossy(&trace.stderr)
                    ));
                }
            }
            let trace_suffix = if trace_fragments.is_empty() {
                String::new()
            } else {
                format!(" [{}]", trace_fragments.join(", "))
            };
            failures.push(format!("{} ({:?}): {}", case.name, case.status, detail));
            if !trace_suffix.is_empty() {
                failures.push(trace_suffix);
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "litmus suite reported failures on {backend:?}: {}",
            failures.join(", ")
        );
    }

    Ok(())
}

#[allow(deprecated)]
fn run_suite_native(driver: &CompilerDriver, sources: &[PathBuf]) -> Result<(), Box<dyn Error>> {
    let target = Target::host();
    let raw_tempdir = Builder::new().prefix("litmus-native").tempdir()?;
    let keep_temp = std::env::var("CHIC_KEEP_LITMUS_TEMP").is_ok();
    let (tempdir_path, _tempdir_guard) = if keep_temp {
        (raw_tempdir.into_path(), None)
    } else {
        (raw_tempdir.path().to_path_buf(), Some(raw_tempdir))
    };
    eprintln!("native litmus tempdir: {:?}", tempdir_path);
    let harness = write_native_harness(&tempdir_path)?;
    let mut inputs = sources.to_vec();
    inputs.push(harness.clone());
    let artifact_stub = native_artifact_path(&tempdir_path, &target);
    let load_stdlib = CompilerDriver::should_load_stdlib(&inputs);
    let report = driver.build(BuildRequest {
        inputs,
        manifest: None,
        workspace: None,
        target: target.clone(),
        kind: ChicKind::Executable,
        backend: Backend::Llvm,
        runtime_backend: RuntimeBackend::Chic,
        output: Some(artifact_stub.clone()),
        run_timeout: None,
        emit_wat_text: false,
        emit_object: false,
        coverage: false,
        cpu_isa: CpuIsaConfig::baseline(),
        emit_header: false,
        emit_library_pack: false,
        cc1_args: Vec::new(),
        cc1_keep_temps: false,
        load_stdlib,
        trace_pipeline: false,
        trait_solver_metrics: false,
        defines: Vec::new(),
        log_level: LogLevel::Info,
        ffi: BuildFfiOptions::default(),
        configuration: "Debug".to_string(),
        framework: None,
        artifacts_path: None,
        obj_dir: None,
        bin_dir: None,
        no_dependencies: false,
        no_restore: false,
        no_incremental: false,
        rebuild: false,
        incremental_validate: false,
        clean_only: false,
        disable_build_servers: false,
        source_root: None,
        properties: Vec::new(),
        verbosity: chic::driver::types::Verbosity::Normal,
        telemetry: chic::driver::types::TelemetrySetting::Auto,
        version_suffix: None,
        nologo: false,
        force: false,
        interactive: false,
        self_contained: None,
        doc_enforcement: MissingDocsRule::default(),
    })?;
    let artifact = report
        .artifact
        .clone()
        .unwrap_or_else(|| artifact_stub.clone());
    let output = Command::new(&artifact).output()?;
    if !output.status.success() {
        let mut message = String::new();
        if !output.stdout.is_empty() {
            message.push_str("stdout:\n");
            message.push_str(&String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            if !message.is_empty() {
                message.push('\n');
            }
            message.push_str("stderr:\n");
            message.push_str(&String::from_utf8_lossy(&output.stderr));
        }
        if message.is_empty() {
            message = format!("exit status {}", output.status);
        }
        return Err(format!("native litmus run failed: {message}").into());
    }
    Ok(())
}

fn native_artifact_path(dir: &Path, target: &Target) -> PathBuf {
    let triple = target.triple().to_string();
    let mut path = dir.join("litmus_suite");
    if triple.contains("windows") && path.extension().is_none() {
        path.set_extension("exe");
    }
    path
}

fn write_native_harness(dir: &Path) -> Result<PathBuf, std::io::Error> {
    let path = dir.join("litmus_harness.cl");
    let mut body = String::from(
        "namespace Tests.Concurrency.Litmus;\n\nimport Std.Platform.IO;\nimport Std.Runtime;\n\npublic static class HarnessMain\n{\n    public static int Main()\n    {\n",
    );
    let tests = [
        "StoreBufferingRejectsZeroZero",
        "LoadBufferingRejectsOneOne",
        "IriwRejectsInconsistentReads",
        "MessagePassingTransfersPublishedValue",
    ];
    for test in tests {
        body.push_str("        Stdout.WriteLine(StringRuntime.FromStr(\"");
        body.push_str(test);
        body.push_str(" start\"));\n");
        body.push_str("        ");
        body.push_str(test);
        body.push_str("();\n");
        body.push_str("        Stdout.WriteLine(StringRuntime.FromStr(\"");
        body.push_str(test);
        body.push_str(" done\"));\n");
    }
    body.push_str("        Stdout.WriteLine(StringRuntime.FromStr(\"litmus done\"));\n");
    body.push_str("        return 0;\n    }\n}\n");
    std::fs::write(&path, body)?;
    Ok(path)
}

#[test]
fn litmus_suite_runs_on_llvm_and_wasm() -> Result<(), Box<dyn Error>> {
    if !litmus_enabled() {
        eprintln!("skipping litmus suite because CHIC_ENABLE_LITMUS is not set");
        return Ok(());
    }
    run_suite(Backend::Llvm)?;
    run_suite(Backend::Wasm)?;
    Ok(())
}
