# Dynamic FFI Imports

This document defines the extended foreign-function interface (FFI) surface for Chic 0.10. It explains how the compiler interprets `@extern` metadata, how the runtime locates shared libraries at execution time, and how tooling/CLI flags influence search paths and packaging. The behaviour described here is normative; the language specification (§6.4 *Foreign Function Interfaces*) links to this document for the operational details.

## 1. Motivation

Chic already supports static `@extern("C")` imports paired with `@link("c")`. Modern applications, however, need richer metadata:

- Bind to platform-specific names (`kernel32.dll` vs `libSystem.dylib`).
- Support alternate calling conventions (`stdcall`, `system`, `fastcall`, etc.).
- Redirect exported names (`alias="InvokeCore"`) and mark optional bindings that may be absent at runtime.
- Configure eager (`dlopen`/`LoadLibrary` at startup) vs lazy (bind on first call) modes with clear, actionable diagnostics.

## 2. Attribute Syntax

`@extern` now accepts a parameter list for configuring dynamic library binding. The canonical grammar is:

```chic
@extern(
    convention = "system",     // optional; default = "C"
    library    = "user32",     // optional; resolves via dynamic loader when present
    alias      = "MessageBoxW",// optional entry alias (cases sensitive)
    binding    = "lazy",       // optional: "lazy" | "eager"
    optional   = true,         // optional: allow missing library/symbol without aborting
    charset    = "utf16"       // optional string marshaling hint for future interop
)
private static extern int MessageBox(
    void* hwnd,
    string text,
    string caption,
    uint type);
```

Short forms remain valid:

```chic
@extern("C")                  // string literal = convention
@extern("C", library = "c")   // positional + named
```

### 2.2 Example

```chic
import Std.Strings;

namespace Interop.Windows;

@extern(
    convention = "system",
    library    = "user32",
    alias      = "MessageBoxW",
    binding    = "lazy",
    optional   = false,
    charset    = "utf16"
)
internal static extern int MessageBox(
    void* hwnd,
    string text,
    string caption,
    uint type);
```

When the compiler encounters this definition it:

1. Records the metadata line\
   `extern:Interop::Windows::MessageBox=convention=system;binding=lazy;library=user32;alias=MessageBoxW;charset=utf16`
2. Emits a per-function descriptor describing the attribute arguments.
3. Generates a Chic stub (`Interop__Windows__MessageBox`) that forwards to the runtime loader. The stub caches the function pointer, marshals the arguments, and returns the native result.

The CLI invocation can then bundle the Windows SDK import library alongside the Chic binary:

```
chic build src/app.ch \
  --ffi-search "%WindowsSdkDir%\\bin\\x64" \
  --ffi-search "./extern" \
  --ffi-default windows={0}.dll \
  --package user32.dll
```

### 2.1 Supported Fields

| Field      | Type      | Default        | Notes |
|------------|-----------|----------------|-------|
| `convention` | string | `"C"`  | `"C"`, `"system"`, `"stdcall"`, `"fastcall"`, `"vectorcall"`. Codegen rejects unsupported conventions per backend. |
| `library`  | string    | _none_         | If omitted, the symbol is expected to resolve statically (`@link`). When set, runtime dynamic loading is used. |
| `alias`    | string    | function name  | Overrides the exported entry name. |
| `binding`  | string    | `"lazy"`       | `"lazy"` defers loading until first call; `"eager"` resolves during module init/startup; `"static"` is only valid when no `library` is provided and otherwise triggers a diagnostic. |
| `optional` | bool      | `false`        | When true, missing libraries/symbols emit `[FFI-W0001]` and return the default value for the return type; when false the runtime aborts with a structured error. |
| `charset`  | string    | `"ansi"`       | Reserved for future marshaling helpers (UTF-8/UTF-16). |

Attributes can be combined with `@link`, but the meanings differ: `@link` influences the static linker, while `@extern(... library=...)` governs runtime lookup.

## 3. Resolution Rules

The compiler records every `@extern` function in the metadata section emitted by the backend. At runtime the loader inspects that metadata, normalises the request, and resolves a function pointer via the OS API. Each binding is cached per descriptor so repeated calls do not pay the loader penalty.

### 3.1 Default Names

The runtime normalizes requested libraries per platform:

| Platform | Default Prefix | Default Suffix | Example (`library="sqlite3"`) |
|----------|----------------|----------------|--------------------------------|
| macOS    | `lib`          | `.dylib`       | `libsqlite3.dylib`             |
| Linux    | `lib`          | `.so`          | `libsqlite3.so`                |
| Windows  | _none_         | `.dll`         | `sqlite3.dll`                  |
| WASI     | _none_         | `.wasm`        | `sqlite3.wasm`                 |

Users can override the derived filename by supplying an explicit extension (e.g., `library="MyLib.framework/Core"`). The CLI additionally exposes:

```
chic build ... \
  --ffi-search /usr/lib \
  --ffi-search ./extern \
  --ffi-default linux=lib{0}.so \
  --ffi-default windows={0}.dll
```

`--ffi-default` lets teams override prefixes/suffixes per platform; `{0}` is replaced with the requested `library` string.

### 3.2 Search Order

1. Directories supplied via `--ffi-search` (embedded into the compiled module).
2. `CHIC_FFI_SEARCH` (path-separated), then `LD_LIBRARY_PATH` (and `DYLD_LIBRARY_PATH` on macOS), and `PATH` on Windows.
3. The executable directory and `<artifact>.deps/` (packaged assets copied by `--ffi-package`).
4. The current working directory.
5. Platform defaults (`/usr/lib`, `/usr/local/lib`, `/System/Library/Frameworks`, `%SystemRoot%\System32`).

### 3.3 Binding Modes

- **Lazy**: The first invocation triggers `dlopen`/`LoadLibrary` + `dlsym`/`GetProcAddress`. The runtime caches the resulting symbol pointer and subsequent calls jump directly to the resolved address. Failures throw a structured exception (`FFIError::MissingLibrary` or `FFIError::MissingSymbol`) unless `optional=true`, in which case the call returns the default value of the return type (zero/null) and emits a warning.
- **Eager**: Libraries/symbols are resolved during startup (before `Main`). This is useful for apps that prefer to fail fast during initialization. When `optional=false`, eager failures abort startup. Optional bindings still resolve lazily even when `binding="eager"` so they do not penalise startup time.

### 3.4 Error Diagnostics

All failures include:

- Requested entry, library, and calling convention.
- Platform-specific loader error (`dlerror()`, `GetLastError()`).
- Suggested remediation (e.g., “ensure `libm.so` is installed or extend `chic build --ffi-search`”).

Optional bindings emit `FFIW0001` warnings at load time so CI can detect missing optional features.

## 4. Runtime Loader API

`src/runtime/ffi.rs` implements platform shims directly on top of the OS loaders:

- Unix targets call `dlopen`/`dlsym` with `RTLD_NOW|RTLD_LOCAL`.
- Windows targets call `LoadLibraryW`/`GetProcAddress` and format `GetLastError()` via `FormatMessageW`.
- Handles and symbol pointers are cached per `ChicFfiDescriptor` so repeated calls do not re-enter the loader.
- Optional bindings log `[FFI-W0001]` to stderr and return null/zero; required bindings `panic!` with a structured message that includes the library, symbol, binding, and convention.

## 5. Tooling & Packaging

### 5.1 CLI Flags

| Flag | Description |
|------|-------------|
| `--ffi-search <path>` | Append directories to the runtime search list. Repeatable. |
| `--ffi-default <os>=<pattern>` | Override prefix/suffix per OS (`{0}` placeholder). |
| `--ffi-package <glob>` | Include matching shared libraries in `chic package` output. |

Each `--ffi-search` argument is canonicalised and embedded into the compiled module; the
runtime invokes `chic_rt_ffi_add_search_path` for every path during startup so
lazy/eager resolutions see the same directory order as the CLI. `--ffi-default` installs
an alternate filename pattern via `chic_rt_ffi_set_default_pattern`. For example,
`--ffi-default macos=/opt/custom/lib{0}.dylib` forces all macOS dynamic bindings to probe
`/opt/custom/lib<name>.dylib` before falling back to the platform defaults. The runtime
continues to respect `CHIC_FFI_SEARCH` and `$PWD`, so deployments can override the CLI at
launch time without recompilation.

`--ffi-package` copies resolved `.dll/.so/.dylib` assets into `<artifact>.deps/` after
linking and adds the same files to `.clrlib` archives under `ffi/<filename>`. The runtime
automatically probes the executable directory and `<artifact>.deps/`, so packaged assets
work even when `--ffi-search` is omitted; the flag remains available to add additional
locations or override deployment layouts.

These flags can also be stored in `manifest.yaml`:

```yaml
ffi:
  search:
    - "./extern"
    - "/usr/local/lib"
  defaults:
    linux: "lib{0}.so"
    macos: "{0}.framework/{0}"
```

### 5.2 `chic extern bind`

`chic extern bind` generates Chic wrapper modules from C headers. It understands
straight-line prototypes (`int foo(int a, char* b);`), pointer parameters (including
`const` qualifiers), and emits Chic-native pointer types (`*mut byte`, `*const byte`).

```
chic extern bind \
  --library sqlite3 \
  --header /usr/include/sqlite3.h \
  --namespace Std.Interop.Sqlite \
  --output packages/std/src/io/sqlite.ch \
  --binding eager \
  --optional
```

- Emits `// <auto-generated by chic extern bind>` header and a namespace declaration.
- Groups functions inside the requested namespace; function identifiers are sanitized.
- Adds `@extern` metadata derived from the CLI options (binding/convention/library/optional).
- Pointer parameters map to Chic raw pointers (`*mut/*const`); POD parameters map to the
  regular numeric/bool types. Unknown identifiers fall back to the C spelling so typedefs
  can be massaged manually.

## 6. Metadata & Symbol Mapping

Each build emits per-function metadata entries so tooling and runtime loaders can audit the
generated artifact. Every `@extern` definition produces a line of the form:

```
extern:Namespace::Func=convention=system;binding=eager;library=user32;alias=MessageBoxW;optional=true;charset=utf16
```

Keys are only emitted when the corresponding field is set. `binding` always appears and uses the same spelling as the source attribute (`static`, `lazy`, `eager`). Tooling can parse these entries to prefetch dynamic libraries, surface diagnostics, or verify that alias names match the expected runtime symbol. LLVM lowering honours `alias=` automatically: when present, the backend emits calls to that symbol instead of the sanitised Chic name. When `library` is omitted the entry is tagged as `binding=static` and no runtime loader involvement is required; the backend simply declares the alias and defers to the platform linker.

## 7. Testing

`tests/dynamic_ffi.rs` drives the end-to-end suite (gated by `CHIC_ENABLE_DYNAMIC_FFI=1`):

- Builds a tiny `ffi_math` shared library with Clang and calls it from Chic code.
- Verifies optional bindings emit `[FFI-W0001]` while returning default values.
- Packages the shared library with `--ffi-package` and confirms resolution works from `<artifact>.deps/` without explicit search flags.
- Asserts required eager bindings fail at startup with the structured “unable to locate library …” diagnostic.

CI pipelines run these tests on all supported RIDs when dynamic FFI is enabled; missing platforms skip with a diagnostic.

## 8. Future Work

- String marshaling helpers controlled by `charset`.
- Automatic struct/enum layout validation against foreign headers.
- Hot reloading / unloading semantics (`dlclose`, `FreeLibrary`) once the runtime can ensure no outstanding pointers remain.

---

*Status: Implemented (runtime/ffi.rs, tests/dynamic_ffi.rs).*
