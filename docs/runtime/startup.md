# Chic Native Startup Runtime

This note outlines the native program bootstrap that accompanies LLVM builds.
It complements the existing Wasm executor by providing a Chic-authored
startup module (`packages/std/src/native_startup.cl`) that is linked into every native
executable built by `chic build --backend llvm`. Additional design context is
captured in the specification and runtime docs.

Std.Platform.IO self-hosting: stdin/stdout/stderr now live in Chic code under
`packages/std/src/io/`. Native startup no longer relies on Rust IO shims; WASM hosts
must register streaming callbacks via the exported
`chic_rt_wasm_io_{register,set_terminals}` symbols before invoking
`chic_main` so buffering/TTY detection stays accurate.

## Entry Shims

`compile_module` now emits metadata describing the Chic entry point and
any compiled `testcase` functions. The native startup module exports a
conventional C `main` symbol that is linked alongside the generated object
code. At runtime the module:

- records the raw `argc`, `argv` and `envp` pointers for later inspection,
- validates the exported startup descriptor and rejects binaries that omit
  `Main`,
- dispatches to the generated Chic entry function (`__chic_program_main`)
  while respecting the declared return type and parameter list, and
- normalises exit codes (unit → 0, `bool` → 0/1, `int` → passthrough).

## Command-Line Arguments and Environment

The bootstrap records argument and environment pointers up front so Chic
code can retrieve the raw process state later. Four helper functions are
exported with default visibility:

```
void    chic_rt_startup_store_state(
            int32_t argc,
            const char **argv,
            const char **envp);
int32_t chic_rt_startup_raw_argc(void);
intptr_t chic_rt_startup_raw_argv(void);
intptr_t chic_rt_startup_raw_envp(void);
```

`chic_rt_startup_store_state` is invoked from the Chic-native startup
module as soon as `main` executes. The three accessors return the cached raw
pointers (encoded as pointer-width integers) so that higher-level libraries can
either walk the argument array manually or hand the values to specialised
runtime helpers. `Std.Runtime.Startup.StartupRuntimeState` provides the Chic
side of this API and exposes the helpers used by `packages/std/src/native_startup.cl`.

### Flag decoding and dispatch

`Std.Runtime.Startup.StartupFlags` now centralises interpretation of the bitfield
emitted by the LLVM backend. The helper exposes strongly-typed queries such as
`EntryIsAsync`, `EntryUsesArgs`, and `EntryUsesEnv`, allowing the native startup
shim to zero out argument/environment pointers when the entry point does not
declare them. This keeps the ABI contract exact—async `Main` and synchronous
entry points both see the same inputs that MIR lowering recorded, and
`--run-tests` dispatch reuses the same helper when deciding whether a testcase
should be routed through the async executor.

### `#![no_main]` crates

- Use `#![no_main]` to disable the implicit `Main` requirement. Startup
  descriptors are suppressed so you can wire your own `start`/entry symbol in a
  bootloader or host runtime.
- The attribute is allowed with `#![no_std]` and `#![std]`. The standard
  prelude (`Std`) remains implicitly imported; only the entry shim is skipped.
- Executables must still expose a runnable entry point for their environment
  (e.g., an exported WASM start function or an `@extern("C") start` symbol).
- Freestanding builds link the no_std runtime shim (`packages/runtime.no_std/src/PanicHandlers.cl`)
  that exports `chic_rt_panic`/`chic_rt_abort` without platform IO.

## Panic and Abort Handling

Native code now links two lightweight termination helpers:

```
void chic_rt_panic(int32_t code); // exits with status 101
void chic_rt_abort(int32_t code); // exits with status 134
```

Both helpers emit a diagnostic to `stderr` before terminating. They provide
deterministic exit semantics comparable to the Wasm runtime hooks and will
integrate with future lowering once unwinding support lands.
`Std.Runtime.Startup.NativeStartup` funnels both exports through a shared
`ExitProcess` helper so the exported surface always terminates the process
cleanly even when invoked directly from host code.

## Test Harness Mode

Metadata emitted by the LLVM backend includes a table of compiled `testcase`
functions. Supplying `--run-tests` on the command line activates a simple test
runner that invokes each testcase in declaration order, reports pass/fail
messages, and returns a non-zero exit code when any testcase fails. Async
testcases are now dispatched through the native async runtime just like async
`Main`, so both synchronous and asynchronous suites behave the same from the
CLI’s perspective.

## Async `Main`

The startup descriptor signals when `Main` was compiled as `async`. The native
shim recognises the flag, invokes the async entry through
`chic_rt_startup_call_entry_async`, and blocks on the returned task via
`chic_rt_startup_complete_entry_async`. The native async runtime drives
the resulting future so async `Main` now executes under the native backend
without requiring the MIR interpreter.

## Async overrides and CLI behaviour

When running with `CHIC_SKIP_STDLIB=1`, the CLI can inject stub startup/async modules via
`CHIC_STARTUP_STDLIB_OVERRIDE` and `CHIC_ASYNC_STDLIB_OVERRIDE`. This path powers the
`tests/backend_validation.rs` async fixtures for LLVM/WASM without requiring a full stdlib build.
Async LLVM testcases surface `[SKIP] ... requires the runtime executor` instead of executing inside
the MIR interpreter; WASM harnesses execute when the runtime hooks are present and otherwise skip.
Both override paths still route through `Std.Async.Runtime.BlockOn` so async `Main` and async tests
remain parity-checked against the runtime executor.
