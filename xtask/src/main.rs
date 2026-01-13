mod an0005;
mod budget;
mod coverage;
mod gpu_kernel_analyze;
mod lint_pedantic;
mod lint_sizes;
mod ll1_guard;
mod manifest_location_guard;
mod metrics;
mod runtime_calls_guard;
mod runtime_shim_guard;
mod runtime_std_guard;
mod runtime_symbol_guard;
mod schedule_tune;
mod shim_state_guard;
mod std_rust_test_guard;
mod sync_lint;
mod task_status;
mod testcase_lint;
mod unicode;
mod wasm_fixtures;

use std::process;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("lint-sizes") => lint_sizes::run(),
        Some("lint-ll1") => ll1_guard::run(),
        Some("lint-pedantic") => lint_pedantic::run(),
        Some("lint-runtime-shim") => runtime_shim_guard::run(),
        Some("lint-runtime-calls") => runtime_calls_guard::run(),
        Some("lint-runtime-stdlib") => runtime_std_guard::run(),
        Some("lint-runtime-symbols") => runtime_symbol_guard::run(),
        Some("lint-shim-state") => shim_state_guard::run(),
        Some("lint-sync") => sync_lint::run(),
        Some("lint-manifest-location") => manifest_location_guard::run(),
        Some("lint-stdlib-rust-tests") => std_rust_test_guard::run(),
        Some("lint-testcases") => testcase_lint::run(),
        Some("an0005") => an0005::run(),
        Some("budget") => {
            let remaining: Vec<String> = args.collect();
            budget::run(&remaining)
        }
        Some("metrics") => {
            let mut bench: Option<String> = None;
            let mut skip_tests = false;
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--bench" => {
                        let name = args
                            .next()
                            .ok_or_else(|| "expected bench name after --bench".to_string())?;
                        bench = Some(name);
                    }
                    "--skip-tests" => skip_tests = true,
                    other => {
                        return Err(format!("unknown metrics flag `{other}`").into());
                    }
                }
            }
            metrics::run(metrics::MetricsOptions { bench, skip_tests })
        }
        Some("coverage") => {
            let mut min_percent = 0.0;
            let mut output: Option<String> = None;
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--min" => {
                        let value = args
                            .next()
                            .ok_or_else(|| "expected value after --min".to_string())?;
                        min_percent = value
                            .parse()
                            .map_err(|_| format!("invalid --min coverage percentage `{value}`"))?;
                    }
                    "--output" => {
                        output = Some(
                            args.next()
                                .ok_or_else(|| "expected path after --output".to_string())?,
                        );
                    }
                    other => return Err(format!("unknown coverage flag `{other}`").into()),
                }
            }
            coverage::run(coverage::CoverageOptions {
                min_percent,
                output: output.map(Into::into),
            })
        }
        Some("task-status") => {
            let remaining: Vec<String> = args.collect();
            task_status::run(&remaining)
        }
        Some("wasm-fixtures") => wasm_fixtures::run(),
        Some("unicode17") => unicode::run(),
        Some("schedule-tune") => schedule_tune::run(),
        Some("gpu-kernel-analyze") => gpu_kernel_analyze::run(),
        Some("help") | None => {
            eprintln!(
                "Available commands:\n  cargo xtask lint-sizes        Check source files against the 1000-line limit.\n  cargo xtask lint-ll1          Ensure parser LL(1) guardrails are documented (LL1_ALLOW comments).\n  cargo xtask lint-pedantic     Run clippy::pedantic and fail on warnings in codegen/parser/syntax modules.\n  cargo xtask lint-runtime-shim Guard against reintroducing the deprecated Rust runtime shim in source code.\n  cargo xtask lint-runtime-calls Freeze chic_rt_* callsites in Rust sources outside src/runtime.\n  cargo xtask lint-runtime-stdlib Freeze chic_rt_* usage across Std/Core/native runtime Chic sources.\n  cargo xtask lint-runtime-symbols Freeze the Rust runtime surface; fail on new chic_rt_* exports.\n  cargo xtask lint-shim-state   Fail if shim files/symbols/references exist; keep native allowlist minimal.\n  cargo xtask lint-sync         Ensure Std synchronisation primitives call into the runtime-backed implementations.\n  cargo xtask lint-manifest-location Ensure manifests only live at package roots (packages/<name>/manifest.yaml).\n  cargo xtask lint-stdlib-rust-tests Fail if Rust sources appear under Std packages.\n  cargo xtask lint-testcases    Enforce Given/When/Then testcase names and single-assert bodies.\n  cargo xtask an0005            Guard against legacy `stdlib`/`StdLib` namespaces/usings.\n  cargo xtask metrics [--bench] Capture clean/incremental compile timings and artifact sizes, or run a specific bench.\n  cargo xtask coverage [--min <percent>] [--output <path>]\n                              Run cargo llvm-cov for the workspace (default min 0%; pass --min to enforce).\n  cargo xtask budget [--update] Verify (or refresh) MIR lowering pass budgets via `tests/lowering_budget`.\n  cargo xtask task-status [--enforce-mm]\n                              Verify diagnostics/spec metadata (optionally enforce MM-series checks).\n  cargo xtask wasm-fixtures     Rebuild the WASM type fixture index from catalog.json.\n  cargo xtask unicode17         Regenerate Unicode 17 identifier/normalization/grapheme tables under generated/unicode17/.\n  cargo xtask schedule-tune    Stub for offline schedule tuning (profile sidecar generation).\n  cargo xtask gpu-kernel-analyze Stub to inspect GPU kernel metadata."
            );
            Ok(())
        }
        Some(other) => Err(format!("unknown xtask command: {other}").into()),
    }
}
