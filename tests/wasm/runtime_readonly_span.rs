#![allow(unused_imports)]

use chic::codegen::{Backend, CpuIsaConfig};
use chic::driver::{BuildRequest, CompilerDriver, ChicKind};
use chic::manifest::MissingDocsRule;
use chic::runtime::execute_wasm;
use chic::Target;
use tempfile::NamedTempFile;

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!("../testdate/", $name))
    };
}

#[test]
#[ignore = "ReadonlySpan runtime coverage for WASM pending"]
fn wasm_readonly_span_smoke() -> Result<(), Box<dyn std::error::Error>> {
    let program = fixture!("readonly_span_exec.ch");
    let temp_src = NamedTempFile::new()?;
    std::fs::write(temp_src.path(), program)?;

    let driver = CompilerDriver::new();
    let temp_bin = tempfile::tempdir()?.path().join("readonly_span.wasm");
    driver.build(BuildRequest {
        inputs: vec![temp_src.path().to_path_buf()],
        manifest: None,
        workspace: None,
        target: Target::host(),
        kind: ChicKind::Executable,
        backend: Backend::Wasm,
        runtime_backend: chic::runtime::backend::RuntimeBackend::Chic,
        output: Some(temp_bin.clone()),
        emit_wat_text: false,
        emit_object: false,
        cpu_isa: chic::codegen::CpuIsaConfig::default(),
        emit_header: false,
        emit_library_pack: false,
        cc1_args: Vec::new(),
        cc1_keep_temps: false,
        load_stdlib: true,
        trace_pipeline: false,
        trait_solver_metrics: false,
        defines: Vec::new(),
        log_level: chic::logging::LogLevel::Info,
        ffi: Default::default(),
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

    let _ = execute_wasm(&std::fs::read(temp_bin)?, "chic_main")?;
    Ok(())
}
