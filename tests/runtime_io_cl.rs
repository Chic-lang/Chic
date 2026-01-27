use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

mod common;
use common::write_source;

#[test]
fn sync_stdin_round_trips_stdout() {
    if std::env::var("CHIC_ENABLE_IO_RUNTIME").is_err() {
        eprintln!(
            "skipping sync_stdin_round_trips_stdout (set CHIC_ENABLE_IO_RUNTIME=1 to enable)"
        );
        return;
    }

    let dir = tempfile::tempdir().expect("temp dir");
    let main_src = dir.path().join("io_roundtrip.ch");

    write_source(
        &main_src,
        r#"
namespace Exec;

import Std.Platform.IO;

public int Main()
{
    var line = Stdin.ReadLine();
    Stdout.WriteLine(line);
    return 0;
}
"#,
    );

    cargo_bin_cmd!("chic")
        .arg("run")
        .arg(&main_src)
        .write_stdin("hello-from-stdin\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello-from-stdin"))
        .stderr(predicate::str::is_empty().or(predicate::str::contains(
            "warning: overriding the module target triple",
        )));
}

#[test]
fn wasm_binary_exposes_io_hooks() {
    if std::env::var("CHIC_ENABLE_IO_RUNTIME").is_err() {
        eprintln!("skipping wasm_binary_exposes_io_hooks (set CHIC_ENABLE_IO_RUNTIME=1 to enable)");
        return;
    }

    use chic::Target;
    use chic::chic_kind::ChicKind;
    use chic::codegen::{Backend, CpuIsaConfig};
    use chic::driver::{BuildRequest, CompilerDriver};
    use chic::manifest::MissingDocsRule;
    use chic::runtime::wasm_executor::{WasmProgram, execute_wasm};
    use tempfile::NamedTempFile;

    let program_src = r#"
namespace Exec;

import Std.Platform.IO;

public int Main()
{
    Stdout.WriteLine("io-hooks");
    return 0;
}
"#;

    let temp_src = NamedTempFile::new().expect("temp src");
    std::fs::write(temp_src.path(), program_src).expect("write source");

    let driver = CompilerDriver::new();
    let temp_bin = tempfile::tempdir()
        .expect("tempdir")
        .path()
        .join("io_hooks.wasm");
    driver
        .build(BuildRequest {
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
            coverage: false,
            cpu_isa: CpuIsaConfig::default(),
            emit_header: false,
            emit_library_pack: false,
            cc1_args: Vec::new(),
            cc1_keep_temps: false,
            load_stdlib: true,
            trace_pipeline: false,
            trait_solver_metrics: false,
            defines: Vec::new(),
            log_level: chic::logging::LogLevel::Warn,
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
        })
        .expect("build wasm");

    let bytes = std::fs::read(temp_bin).expect("read wasm");
    let program = WasmProgram::from_bytes(&bytes).expect("parse wasm");
    assert!(
        program.has_export("chic_rt_wasm_io_register")
            && program.has_export("chic_rt_wasm_io_set_terminals"),
        "wasm exports must include IO registration hooks"
    );

    let outcome = execute_wasm(&bytes, "chic_main").expect("execute wasm");
    let stdout = String::from_utf8_lossy(&outcome.trace.stdout);
    assert!(
        stdout.contains("io-hooks"),
        "stdout should capture wasm writes; got `{stdout}`"
    );
}
