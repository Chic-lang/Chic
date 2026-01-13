use criterion::{Criterion, criterion_group, criterion_main};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

use chic::ChicKind;
use chic::codegen::{Backend, CpuIsaConfig};
use chic::driver::{BuildRequest, CompilerDriver};
use chic::logging::LogLevel;
use chic::manifest::MissingDocsRule;
use chic::target::Target;

fn chic_sample_source() -> &'static str {
    r"
namespace Bench;

public int Fibonacci(int n)
{
    if (n <= 1) { return n; }
    return Fibonacci(n - 1) + Fibonacci(n - 2);
}

public int Main()
{
    return Fibonacci(12);
}
"
}

fn rust_sample_source() -> &'static str {
    r"fn fibonacci(n: i32) -> i32 {
    if n <= 1 { return n; }
    fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    std::process::exit((fibonacci(12)) as i32);
}
"
}

fn bench_chic(c: &mut Criterion) {
    let driver = CompilerDriver::new();
    let target = Target::host();
    let program = chic_sample_source();

    c.bench_function("chic_wasm_build", |b| {
        b.iter(|| {
            let dir = tempdir().expect("tempdir");
            let src_path = dir.path().join("bench.cl");
            fs::write(&src_path, program).expect("write source");
            let output_path = dir.path().join("bench.clbin");
            driver
                .build(BuildRequest {
                    inputs: vec![src_path.clone()],
                    manifest: None,
                    workspace: None,
                    target: target.clone(),
                    kind: ChicKind::Executable,
                    backend: Backend::Wasm,
                    runtime_backend: chic::runtime::backend::RuntimeBackend::Chic,
                    output: Some(output_path),
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
                    log_level: LogLevel::Info,
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
                .expect("compile");
        });
    });
}

fn bench_rustc(c: &mut Criterion) {
    if Command::new("rustc").arg("--version").output().is_err() {
        return;
    }
    let program = rust_sample_source();

    c.bench_function("rustc_compile", |b| {
        b.iter(|| {
            let dir = tempdir().expect("tempdir");
            let src_path = dir.path().join("bench.rs");
            fs::write(&src_path, program).expect("write rust source");
            let output_path = dir.path().join("bench_rust");
            let status = Command::new("rustc")
                .arg(&src_path)
                .arg("-O")
                .arg("-o")
                .arg(&output_path)
                .status()
                .expect("run rustc");
            assert!(status.success());
        });
    });
}

fn bench_csharp(c: &mut Criterion) {
    if Command::new("csc").arg("/help").output().is_err() {
        return;
    }

    let program = r"using Std;
static class Program {
    static int Fibonacci(int n) => n <= 1 ? n : Fibonacci(n - 1) + Fibonacci(n - 2);
    public static int Main() => Fibonacci(12);
}
";

    c.bench_function("csharp_compile", |b| {
        b.iter(|| {
            let dir = tempdir().expect("tempdir");
            let src_path = dir.path().join("Bench.cs");
            fs::write(&src_path, program).expect("write csharp source");
            let output = dir.path().join("Bench.exe");
            let status = Command::new("csc")
                .arg(&src_path)
                .arg(format!("/out:{}", output.display()))
                .status()
                .expect("csc build");
            assert!(status.success());
        });
    });
}

fn benchmarks(c: &mut Criterion) {
    bench_chic(c);
    bench_rustc(c);
    bench_csharp(c);
}

criterion_group!(codegen, benchmarks);
criterion_main!(codegen);
