# Codegen execution test suites

The `tests/codegen_exec` integration crate is now split into three tagged suites:

- `Category::Happy` – success-path samples (LLVM + WASM) that verify round-tripping binaries and async entrypoints.
- `Category::Error` – harness-driven test runners that must report failing cases and surface diagnostics.
- `Category::Perf` – CLI/timeout smoke tests for large fixtures; opt-in via `CHIC_ENABLE_CODEGEN_PERF=1`.

Enable codegen execution locally with `CHIC_ENABLE_CODEGEN_EXEC=1`. Performance-tagged tests require both `CHIC_ENABLE_CODEGEN_EXEC=1` and `CHIC_ENABLE_CODEGEN_PERF=1`; LLVM paths also require `clang` on `PATH`.

## Harness usage

Use `ExecHarness::{wasm,llvm}(Category::…)` to build and run fixtures. Each harness enforces the env/clang gates and returns `HarnessError::Skip` when prerequisites are missing. Convert skips into passing tests via `err.into_test_result(&harness)`.

```rust
let harness = ExecHarness::wasm(Category::Happy);
let artifact = match harness.build_executable(fixture!("wasm_simple_add.cl"), Some("wasm")) {
    Ok(artifact) => artifact,
    Err(err) => return err.into_test_result(&harness),
};
let wasm_bytes = std::fs::read(artifact.output.path())?;
let outcome = chic::runtime::execute_wasm(&wasm_bytes, "chic_main")?;
assert_eq!(outcome.exit_code, 0);
```

The harness exposes `run_tests` for test-runner fixtures; use `Category::Error` with the appropriate backend.

## Adding new cases

1. Drop fixtures under `tests/testdate/` (or `tests/spec/` if they are spec-driven).
2. Choose a category (`happy.rs`, `error.rs`, or `perf.rs`) and drive the fixture through the shared harness.
3. Map skips with `err.into_test_result(&harness)` so CI environments without codegen flags do not fail.
4. For new perf cases, keep them short and guarded; prefer harness unit tests when possible.

Run `cargo test --test codegen_exec -- --nocapture` with the relevant env flags to exercise the suites; capture coverage with:

```
CHIC_ENABLE_CODEGEN_EXEC=1 cargo llvm-cov --test codegen_exec --json --output-path coverage/codegen_exec_local.json -- --nocapture
```
