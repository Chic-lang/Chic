use std::fmt::Write;

#[derive(Debug, Clone)]
struct OptionGuide {
    flag: &'static str,
    description: &'static str,
}

#[derive(Debug, Clone)]
struct CommandGuide {
    names: &'static [&'static str],
    summary: &'static str,
    usage: &'static [&'static str],
    options: &'static [OptionGuide],
    examples: &'static [&'static str],
    docs: &'static [&'static str],
}

const GLOBAL_OPTIONS: &[OptionGuide] = &[
    OptionGuide {
        flag: "-h, --help",
        description: "Show contextual help information.",
    },
    OptionGuide {
        flag: "--version",
        description: "Print Chic version and build metadata.",
    },
    OptionGuide {
        flag: "-c, --configuration <name>",
        description: "Set a configuration/profile (applies to build/run/test unless overridden).",
    },
    OptionGuide {
        flag: "-f, --framework <name>",
        description: "Select a target framework/profile; forwarded to builds.",
    },
    OptionGuide {
        flag: "-r, --runtime <runtime>",
        description: "Default runtime flavour or backend (llvm, wasm, native-std, native-no_std).",
    },
    OptionGuide {
        flag: "-v, --verbosity <level>",
        description: "Global log verbosity (quiet, minimal, normal, detailed, diagnostic).",
    },
    OptionGuide {
        flag: "-p, --property:<name>=<value>",
        description: "Global property override forwarded to the build (repeatable).",
    },
];

const GENERAL_DOCS: &[&str] = &[
    "README.md#Getting-Started",
    "docs/README.md",
    "SPEC.md",
    "docs/manifest_manifest.md",
    "docs/getting-started.md",
    "docs/language/tour.md",
    "docs/troubleshooting.md",
    "docs/style-guide.md",
    "docs/std/README.md",
    "docs/runtime/README.md",
    "docs/guides/logging.md",
    "docs/cli/README.md",
    "docs/tooling/documentation.md",
    "docs/tooling/xml_to_markdown.md",
];

const COMMAND_GUIDES: &[CommandGuide] = &[
    CommandGuide {
        names: &["init"],
        summary: "Create a new Chic project from a named template.",
        usage: &["chic init [path] --template <name> [--name <project>]"],
        options: &[
            OptionGuide {
                flag: "--template <name>",
                description: "Template to instantiate (app, app-console, console).",
            },
            OptionGuide {
                flag: "--name <project>",
                description: "Project/package name (defaults to output directory or template default).",
            },
            OptionGuide {
                flag: "-o, --output <path>",
                description: "Destination directory for generated files (defaults to current directory).",
            },
        ],
        examples: &[
            "chic init --template app MyApp",
            "chic init --template app --name SampleApp --output samples/app",
        ],
        docs: &[
            "docs/getting-started.md",
            "docs/manifest_manifest.md",
            "SPEC.md",
        ],
    },
    CommandGuide {
        names: &["check"],
        summary: "Parse and type-check a source file without emitting code.",
        usage: &["chic check <file> [options]"],
        options: &[
            OptionGuide {
                flag: "-t, --target <triple>",
                description: "Compile for the given target triple (defaults to host).",
            },
            OptionGuide {
                flag: "--crate-type <kind>",
                description: "Override the crate type (exe, staticlib, dylib).",
            },
            OptionGuide {
                flag: "--consteval-fuel <n>",
                description: "Override the const-eval fuel limit for this invocation.",
            },
            OptionGuide {
                flag: "--ffi-search <path>",
                description: "Add <path> to the runtime dynamic library search list (repeatable).",
            },
            OptionGuide {
                flag: "--ffi-default <os>=<pattern>",
                description: "Override default library name pattern for <os> (macos, linux, windows, wasi, any).",
            },
            OptionGuide {
                flag: "--ffi-package <glob>",
                description: "Copy matching shared libraries next to the built artifact and bundle them into .clrlib archives.",
            },
            OptionGuide {
                flag: "--log-format <format>",
                description: "Select log output format (auto, text, json).",
            },
            OptionGuide {
                flag: "--log-level <level>",
                description: "Set log verbosity (error, warn, info, debug, trace).",
            },
            OptionGuide {
                flag: "--error-format <format>",
                description: "Select diagnostic format (human, json, toon, short); defaults depend on TTY.",
            },
            OptionGuide {
                flag: "--trace-pipeline",
                description: "Emit structured tracing spans for pipeline stages.",
            },
            OptionGuide {
                flag: "--trait-solver-metrics",
                description: "Print and log trait-solver metrics even without --trace-pipeline.",
            },
        ],
        examples: &[
            "chic check examples/hello.ch",
            "chic check main.ch --target x86_64-unknown-linux-gnu",
        ],
        docs: &["SPEC.md", "docs/guides/logging.md"],
    },
    CommandGuide {
        names: &["lint"],
        summary: "Run Clippy-style lint passes with workspace configuration and suppression support.",
        usage: &["chic lint <file> [options]"],
        options: &[
            OptionGuide {
                flag: "-t, --target <triple>",
                description: "Compile for the given target triple (defaults to host).",
            },
            OptionGuide {
                flag: "--crate-type <kind>",
                description: "Override the crate type (exe, staticlib, dylib).",
            },
            OptionGuide {
                flag: "--consteval-fuel <n>",
                description: "Override the const-eval fuel limit for this invocation.",
            },
            OptionGuide {
                flag: "--log-format <format>",
                description: "Select log output format (auto, text, json).",
            },
            OptionGuide {
                flag: "--log-level <level>",
                description: "Set log verbosity (error, warn, info, debug, trace).",
            },
            OptionGuide {
                flag: "--error-format <format>",
                description: "Select diagnostic format (human, json, toon, short); defaults depend on TTY.",
            },
            OptionGuide {
                flag: "--trace-pipeline",
                description: "Emit structured tracing spans for pipeline stages.",
            },
            OptionGuide {
                flag: "--trait-solver-metrics",
                description: "Print and log trait-solver metrics even without --trace-pipeline.",
            },
        ],
        examples: &[
            "chic lint examples/hello.ch",
            "chic lint src/main.ch --crate-type dylib",
        ],
        docs: &["docs/cli/linting.md", "SPEC.md"],
    },
    CommandGuide {
        names: &["build", "publish", "pack"],
        summary: "Compile a Chic project (manifest.yaml) using the selected backend.",
        usage: &[
            "chic build [project|directory|file] [options]",
            "chic publish [project|directory|file] [options]",
            "chic pack [project|directory|file] [options]",
        ],
        options: &[
            OptionGuide {
                flag: "-o, --output <path>",
                description: "Write the produced artifact to <path>.",
            },
            OptionGuide {
                flag: "--artifacts-path <dir>",
                description: "Write build outputs and intermediates under <dir>/obj/<target>/<configuration>/<backend>/<runtime> (bin/... for linked artifacts); defaults to workspace ./obj.",
            },
            OptionGuide {
                flag: "-a, --arch <arch>",
                description: "Target architecture (x86_64|amd64 or aarch64|arm64).",
            },
            OptionGuide {
                flag: "--os <os>",
                description: "Target operating system (macos, linux, windows, none).",
            },
            OptionGuide {
                flag: "-t, --target <triple>",
                description: "Compile for the given target triple (defaults to host).",
            },
            OptionGuide {
                flag: "--crate-type <kind>",
                description: "Choose the crate type (exe, staticlib, dylib).",
            },
            OptionGuide {
                flag: "-c, --configuration <name>",
                description: "Configuration/profile name (Debug [default], Release, or custom).",
            },
            OptionGuide {
                flag: "-f, --framework <name>",
                description: "Select a framework or target profile defined by the project/workspace.",
            },
            OptionGuide {
                flag: "--backend <backend>",
                description: "Select backend (llvm, wasm, cc1).",
            },
            OptionGuide {
                flag: "--runtime <runtime>",
                description: "Target runtime flavour (llvm, wasm, native-std, native-no_std).",
            },
            OptionGuide {
                flag: "--runtime-backend <runtime>",
                description: "Execution runtime backend (chic only; Rust shim removed).",
            },
            OptionGuide {
                flag: "--ucr, --use-current-runtime",
                description: "Use the current host arch/OS/runtime triple.",
            },
            OptionGuide {
                flag: "--sc, --self-contained",
                description: "Bundle runtime/stdlib artifacts for executables.",
            },
            OptionGuide {
                flag: "--no-self-contained",
                description: "Rely on shared runtimes (not supported for wasm targets).",
            },
            OptionGuide {
                flag: "--emit=obj",
                description: "Emit object files and skip the final link step (LLVM backend only).",
            },
            OptionGuide {
                flag: "--cpu-isa <list>",
                description: "Comma-separated ISA tiers or 'auto' for host detection.",
            },
            OptionGuide {
                flag: "--sve-bits <bits>",
                description: "Pin SVE vector length (multiple of 128).",
            },
            OptionGuide {
                flag: "--emit-wat",
                description: "Emit a textual .wat module alongside Wasm binaries.",
            },
            OptionGuide {
                flag: "--emit-header",
                description: "Emit a C-compatible header when building libraries.",
            },
            OptionGuide {
                flag: "--emit-lib",
                description: "Bundle compiled objects into a reusable .clrlib archive.",
            },
            OptionGuide {
                flag: "--no-dependencies",
                description: "Build only the current package; skip dependency restore/build.",
            },
            OptionGuide {
                flag: "--no-restore",
                description: "Skip restoring/fetching dependencies (use existing cache only).",
            },
            OptionGuide {
                flag: "--no-incremental",
                description: "Force a clean build; disable incremental caches.",
            },
            OptionGuide {
                flag: "--disable-build-servers",
                description: "Disable build daemons/servers; run in-process only.",
            },
            OptionGuide {
                flag: "--source <path>",
                description: "Override the source root when building a manifest.",
            },
            OptionGuide {
                flag: "--cc1-arg <arg>",
                description: "Forward a raw argument to the cc1 backend (repeatable).",
            },
            OptionGuide {
                flag: "--cc1-keep-input",
                description: "Keep the generated .i file when using --backend cc1.",
            },
            OptionGuide {
                flag: "--consteval-fuel <n>",
                description: "Override the const-eval fuel limit for this invocation.",
            },
            OptionGuide {
                flag: "-p, --property:<name>=<value>",
                description: "Override manifest/build properties (repeatable).",
            },
            OptionGuide {
                flag: "--ffi-search <path>",
                description: "Add <path> to the runtime dynamic library search list (repeatable).",
            },
            OptionGuide {
                flag: "--ffi-default <os>=<pattern>",
                description: "Override default library name pattern for <os> (macos, linux, windows, wasi, any).",
            },
            OptionGuide {
                flag: "--ffi-package <glob>",
                description: "Copy matching shared libraries next to the temporary artifact before execution.",
            },
            OptionGuide {
                flag: "--log-format <format>",
                description: "Select log output format (auto, text, json).",
            },
            OptionGuide {
                flag: "--log-level <level>",
                description: "Set log verbosity (error, warn, info, debug, trace).",
            },
            OptionGuide {
                flag: "-v, --verbosity <level>",
                description: "Log verbosity: quiet, minimal, normal, detailed, diagnostic.",
            },
            OptionGuide {
                flag: "--error-format <format>",
                description: "Select diagnostic format (human, json, toon, short); defaults depend on TTY.",
            },
            OptionGuide {
                flag: "--trace-pipeline",
                description: "Emit structured tracing spans for pipeline stages.",
            },
            OptionGuide {
                flag: "--trait-solver-metrics",
                description: "Enable trait solver telemetry (logs + CLI summary).",
            },
            OptionGuide {
                flag: "--tl:<auto|on|off>",
                description: "Toggle build telemetry collection (perf.json, perf.folded).",
            },
            OptionGuide {
                flag: "--version-suffix <suffix>",
                description: "Append a version suffix when stamping artifacts/metadata.",
            },
            OptionGuide {
                flag: "--nologo",
                description: "Suppress CLI banners/branding.",
            },
            OptionGuide {
                flag: "--force",
                description: "Force rebuild or overwriting stale artifacts when safe.",
            },
            OptionGuide {
                flag: "--interactive",
                description: "Allow interactive prompts for missing dependencies or profile regen.",
            },
        ],
        examples: &[
            "chic build                              # builds manifest.yaml in the current directory",
            "chic build manifest.yaml -c Release",
            "chic build path/to/app --framework wasm32 --property:Version=1.2.3",
            "chic build examples/hello.ch",
        ],
        docs: &[
            "docs/manifest_manifest.md",
            "docs/cli/README.md",
            "docs/wasm_backend.md",
            "docs/guides/logging.md",
            "docs/runtime/dynamic_ffi.md",
        ],
    },
    CommandGuide {
        names: &["clean"],
        summary: "Delete build outputs and intermediates (obj/bin) for a workspace or project.",
        usage: &["chic clean [project|directory|file] [options]"],
        options: &[
            OptionGuide {
                flag: "--artifacts-path <dir>",
                description: "Clean outputs under <dir>/obj and <dir>/bin (defaults to workspace root).",
            },
            OptionGuide {
                flag: "-c, --configuration <name>",
                description: "Configuration/profile to clean (Debug [default], Release, or custom).",
            },
            OptionGuide {
                flag: "--all",
                description: "Remove obj/ and bin/ entirely instead of a single configuration.",
            },
            OptionGuide {
                flag: "--dry-run",
                description: "Print paths that would be removed without deleting anything.",
            },
        ],
        examples: &[
            "chic clean",
            "chic clean -c Release",
            "chic clean path/to/project --all",
            "chic clean --artifacts-path /tmp/chic --dry-run",
        ],
        docs: &["docs/cli/README.md", "docs/manifest_manifest.md"],
    },
    CommandGuide {
        names: &["doc", "docs"],
        summary: "Generate Markdown documentation from XML doc comments.",
        usage: &["chic doc [manifest.yaml] [options]"],
        options: &[
            OptionGuide {
                flag: "-o, --output <dir>",
                description: "Output directory for generated Markdown (default: docs/api).",
            },
            OptionGuide {
                flag: "--layout <single|per-type>",
                description: "Choose single-file or per-type output layout.",
            },
            OptionGuide {
                flag: "--template <path>",
                description: "Markdown template controlling page layout and sections.",
            },
            OptionGuide {
                flag: "--front-matter <path>",
                description: "Front-matter template injected at the top of each page.",
            },
            OptionGuide {
                flag: "--tag-handler <name>",
                description: "Register a custom XML tag handler (repeatable).",
            },
            OptionGuide {
                flag: "--link-resolver <name>",
                description: "Custom link resolver for <see>/<seealso> cref targets.",
            },
            OptionGuide {
                flag: "--no-banner",
                description: "Omit the auto-generated regeneration banner.",
            },
        ],
        examples: &[
            "chic doc",
            "chic doc --output docs/api --layout single --template docs/templates/api.md",
            "chic build --doc-markdown",
        ],
        docs: &[
            "docs/tooling/documentation.md",
            "docs/tooling/xml_to_markdown.md",
            "docs/manifest_manifest.md",
        ],
    },
    CommandGuide {
        names: &["cc1"],
        summary: "Invoke the cc1 stage on a preprocessed C translation unit.",
        usage: &["chic cc1 <file> [options]"],
        options: &[
            OptionGuide {
                flag: "-o, --output <path>",
                description: "Write the generated assembly to <path> (default: replace .i with .s).",
            },
            OptionGuide {
                flag: "-t, --target <triple>",
                description: "Assemble for the given target triple (defaults to host).",
            },
            OptionGuide {
                flag: "--cc1-arg <arg>",
                description: "Forward a raw argument to clang -cc1 (repeatable).",
            },
        ],
        examples: &[
            "chic cc1 module.i",
            "chic cc1 module.i -o module.s --cc1-arg -debug-info-kind=line-tables-only",
        ],
        docs: &["docs/cc1_stage.md"],
    },
    CommandGuide {
        names: &["run"],
        summary: "Build a project (manifest.yaml) and run the resulting executable.",
        usage: &["chic run [project|directory|file] [options]"],
        options: &[
            OptionGuide {
                flag: "-c, --configuration <name>",
                description: "Configuration/profile name (Debug [default], Release, or custom).",
            },
            OptionGuide {
                flag: "-f, --framework <name>",
                description: "Select a framework or target profile defined by the project/workspace.",
            },
            OptionGuide {
                flag: "-t, --target <triple>",
                description: "Compile for the given target triple (defaults to host).",
            },
            OptionGuide {
                flag: "--crate-type <kind>",
                description: "Override the crate type (must remain executable).",
            },
            OptionGuide {
                flag: "--backend <backend>",
                description: "Select backend (llvm, wasm).",
            },
            OptionGuide {
                flag: "--runtime <runtime>",
                description: "Target runtime flavour (llvm, wasm, native-std, native-no_std).",
            },
            OptionGuide {
                flag: "--runtime-backend <runtime>",
                description: "Execution runtime backend (chic only; Rust shim removed).",
            },
            OptionGuide {
                flag: "--cpu-isa <list>",
                description: "Comma-separated ISA tiers or 'auto' for host detection.",
            },
            OptionGuide {
                flag: "--sve-bits <bits>",
                description: "Pin SVE vector length (multiple of 128).",
            },
            OptionGuide {
                flag: "--ffi-search <path>",
                description: "Add <path> to the runtime dynamic library search list (repeatable).",
            },
            OptionGuide {
                flag: "--ffi-default <os>=<pattern>",
                description: "Override default library name pattern for <os> (macos, linux, windows, wasi, any).",
            },
            OptionGuide {
                flag: "--ffi-package <glob>",
                description: "Copy matching shared libraries next to the temporary artifact before executing tests.",
            },
            OptionGuide {
                flag: "--source <path>",
                description: "Override the source root when running a manifest-based project.",
            },
            OptionGuide {
                flag: "--no-dependencies",
                description: "Skip dependency traversal/build for the current invocation.",
            },
            OptionGuide {
                flag: "--no-restore",
                description: "Avoid downloading/restoring dependencies; use existing cache only.",
            },
            OptionGuide {
                flag: "--no-incremental",
                description: "Force a clean rebuild of the project before executing.",
            },
            OptionGuide {
                flag: "--disable-build-servers",
                description: "Run builds in-process without build daemons/servers.",
            },
            OptionGuide {
                flag: "-p, --property:<name>=<value>",
                description: "Override manifest/build properties (repeatable).",
            },
            OptionGuide {
                flag: "-v, --verbosity <level>",
                description: "Log verbosity: quiet, minimal, normal, detailed, diagnostic.",
            },
            OptionGuide {
                flag: "--tl:<auto|on|off>",
                description: "Toggle build telemetry collection (perf.json, perf.folded).",
            },
            OptionGuide {
                flag: "--version-suffix <suffix>",
                description: "Append a version suffix when stamping artifacts/metadata.",
            },
            OptionGuide {
                flag: "--nologo",
                description: "Suppress CLI banners/branding.",
            },
            OptionGuide {
                flag: "--force",
                description: "Force rebuild or overwriting stale artifacts when safe.",
            },
            OptionGuide {
                flag: "--interactive",
                description: "Allow interactive prompts for missing dependencies or profile regen.",
            },
            OptionGuide {
                flag: "--sc, --self-contained",
                description: "Bundle runtime/stdlib artifacts for executables.",
            },
            OptionGuide {
                flag: "--no-self-contained",
                description: "Rely on shared runtimes (not supported for wasm targets).",
            },
            OptionGuide {
                flag: "--consteval-fuel <n>",
                description: "Override the const-eval fuel limit for this invocation.",
            },
            OptionGuide {
                flag: "--log-format <format>",
                description: "Select log output format (auto, text, json).",
            },
            OptionGuide {
                flag: "--log-level <level>",
                description: "Set log verbosity (error, warn, info, debug, trace).",
            },
            OptionGuide {
                flag: "--error-format <format>",
                description: "Select diagnostic format (human, json, toon, short); defaults depend on TTY.",
            },
            OptionGuide {
                flag: "--trace-pipeline",
                description: "Emit structured tracing spans for pipeline stages.",
            },
            OptionGuide {
                flag: "--trait-solver-metrics",
                description: "Enable trait solver telemetry (logs + CLI summary).",
            },
        ],
        examples: &[
            "chic run                               # runs the project in the current directory",
            "chic run manifest.yaml -c Release",
            "chic run path/to/project --framework wasm32 --property:FeatureFlag=on",
            "chic run examples/hello.ch",
        ],
        docs: &[
            "README.md#Getting-Started",
            "docs/cli/README.md",
            "docs/guides/logging.md",
            "docs/runtime/dynamic_ffi.md",
        ],
    },
    CommandGuide {
        names: &["profile"],
        summary: "Run a project under the built-in profiler (perf.json + optional flamegraph).",
        usage: &["chic profile [project|directory|file] [options]"],
        options: &[
            OptionGuide {
                flag: "--profile-out <path>",
                description: "Write profiling output to <path> (default: profiling/latest/perf.json).",
            },
            OptionGuide {
                flag: "--profile-sample-ms <ms>",
                description: "Sampling interval in milliseconds (default: 1).",
            },
            OptionGuide {
                flag: "--profile-flamegraph",
                description: "Render a flamegraph (perf.svg) from the captured profile.",
            },
        ],
        examples: &[
            "chic profile",
            "chic profile --profile-out profiling/run1/perf.json",
            "chic profile --profile-flamegraph",
        ],
        docs: &["docs/tooling/perf_json.md", "docs/cli/performance.md"],
    },
    CommandGuide {
        names: &["test"],
        summary: "Discover and execute tests from a project (manifest.yaml) or source file.",
        usage: &["chic test [project|directory|file] [options]"],
        options: &[
            OptionGuide {
                flag: "-c, --configuration <name>",
                description: "Configuration/profile name (Debug [default], Release, or custom).",
            },
            OptionGuide {
                flag: "-f, --framework <name>",
                description: "Select a framework or target profile defined by the project/workspace.",
            },
            OptionGuide {
                flag: "-t, --target <triple>",
                description: "Compile for the given target triple (defaults to host).",
            },
            OptionGuide {
                flag: "--crate-type <kind>",
                description: "Override the crate type used for test discovery.",
            },
            OptionGuide {
                flag: "--backend <backend>",
                description: "Select backend (llvm, wasm).",
            },
            OptionGuide {
                flag: "--runtime <runtime>",
                description: "Target runtime flavour (llvm, wasm, native-std, native-no_std).",
            },
            OptionGuide {
                flag: "--runtime-backend <runtime>",
                description: "Execution runtime backend (chic only; Rust shim removed).",
            },
            OptionGuide {
                flag: "--cpu-isa <list>",
                description: "Comma-separated ISA tiers or 'auto' for host detection.",
            },
            OptionGuide {
                flag: "--sve-bits <bits>",
                description: "Pin SVE vector length (multiple of 128).",
            },
            OptionGuide {
                flag: "--consteval-fuel <n>",
                description: "Override the const-eval fuel limit for this invocation.",
            },
            OptionGuide {
                flag: "--source <path>",
                description: "Override the source root when testing a manifest-based project.",
            },
            OptionGuide {
                flag: "--log-format <format>",
                description: "Select log output format (auto, text, json).",
            },
            OptionGuide {
                flag: "--log-level <level>",
                description: "Set log verbosity (error, warn, info, debug, trace).",
            },
            OptionGuide {
                flag: "--error-format <format>",
                description: "Select diagnostic format (human, json, toon, short); defaults depend on TTY.",
            },
            OptionGuide {
                flag: "--trace-pipeline",
                description: "Emit structured tracing spans for pipeline stages.",
            },
            OptionGuide {
                flag: "--trait-solver-metrics",
                description: "Enable trait solver telemetry (logs + CLI summary).",
            },
            OptionGuide {
                flag: "--no-dependencies",
                description: "Skip dependency traversal/build for the current invocation.",
            },
            OptionGuide {
                flag: "--no-restore",
                description: "Avoid downloading/restoring dependencies; use existing cache only.",
            },
            OptionGuide {
                flag: "--no-incremental",
                description: "Force a clean rebuild of the project before testing.",
            },
            OptionGuide {
                flag: "--disable-build-servers",
                description: "Run builds in-process without build daemons/servers.",
            },
            OptionGuide {
                flag: "-p, --property:<name>=<value>",
                description: "Override manifest/build properties (repeatable).",
            },
            OptionGuide {
                flag: "-v, --verbosity <level>",
                description: "Log verbosity: quiet, minimal, normal, detailed, diagnostic.",
            },
            OptionGuide {
                flag: "--tl:<auto|on|off>",
                description: "Toggle build telemetry collection (perf.json, perf.folded).",
            },
            OptionGuide {
                flag: "--version-suffix <suffix>",
                description: "Append a version suffix when stamping artifacts/metadata.",
            },
            OptionGuide {
                flag: "--nologo",
                description: "Suppress CLI banners/branding.",
            },
            OptionGuide {
                flag: "--force",
                description: "Force rebuild or overwriting stale artifacts when safe.",
            },
            OptionGuide {
                flag: "--interactive",
                description: "Allow interactive prompts for missing dependencies or profile regen.",
            },
            OptionGuide {
                flag: "--sc, --self-contained",
                description: "Bundle runtime/stdlib artifacts for executables.",
            },
            OptionGuide {
                flag: "--no-self-contained",
                description: "Rely on shared runtimes (not supported for wasm targets).",
            },
            OptionGuide {
                flag: "--test <pattern>",
                description: "Run a single testcase or a wildcard match (repeatable).",
            },
            OptionGuide {
                flag: "--test-group <pattern>",
                description: "Filter by category/tag or namespace prefix; accepts wildcards.",
            },
            OptionGuide {
                flag: "--all",
                description: "Clear filters and run the full suite even if env vars are set.",
            },
            OptionGuide {
                flag: "--test-parallel <N>",
                description: "Override test parallelism (0/absent = auto).",
            },
            OptionGuide {
                flag: "--fail-fast",
                description: "Stop scheduling after the first observed failure.",
            },
            OptionGuide {
                flag: "--watchdog <steps>",
                description: "Set the test watchdog step limit (0 disables).",
            },
            OptionGuide {
                flag: "--watchdog-timeout <ms>",
                description: "Set a wall-clock watchdog timeout per test.",
            },
        ],
        examples: &[
            "chic test                              # tests the project in the current directory",
            "chic test manifest.yaml --test-group smoke --test-parallel 4",
            "chic test path/to/project --configuration Release",
            "chic test suite.ch --backend wasm",
        ],
        docs: &[
            "README.md#Getting-Started",
            "docs/cli/README.md",
            "docs/guides/logging.md",
            "docs/runtime/dynamic_ffi.md",
        ],
    },
    CommandGuide {
        names: &["coverage"],
        summary: "Run tests with coverage collection and optional gating.",
        usage: &["chic coverage [project|directory|file] [options]"],
        options: &[
            OptionGuide {
                flag: "--coverage",
                description: "Enable coverage collection for the run (implied for chic coverage).",
            },
            OptionGuide {
                flag: "--coverage-min <percent>",
                description: "Fail the command if coverage drops below <percent> (0-100).",
            },
        ],
        examples: &["chic coverage", "chic coverage --coverage-min 90"],
        docs: &["docs/coverage.md"],
    },
    CommandGuide {
        names: &["format"],
        summary: "Normalise Chic source formatting in-place.",
        usage: &[
            "chic format [<file|dir> ...]",
            "chic format --stdin [--stdout]",
        ],
        options: &[
            OptionGuide {
                flag: "--config <path>",
                description: "Override the format config (defaults to manifest.yaml).",
            },
            OptionGuide {
                flag: "--check",
                description: "Do not write changes; exit non-zero if reformatting is needed.",
            },
            OptionGuide {
                flag: "--diff",
                description: "Show a unified diff for files that would change.",
            },
            OptionGuide {
                flag: "--write",
                description: "Apply changes to disk (default).",
            },
            OptionGuide {
                flag: "--stdin",
                description: "Format source from stdin instead of files.",
            },
            OptionGuide {
                flag: "--stdout",
                description: "Write formatted output to stdout instead of files.",
            },
        ],
        examples: &[
            "chic format                         # formats sources from manifest.yaml",
            "chic format src tests --check --diff",
            "chic format --stdin --stdout",
        ],
        docs: &["README.md#Getting-Started", "docs/tooling/formatter.md"],
    },
    CommandGuide {
        names: &["mir-dump"],
        summary: "Pretty-print the lowered MIR for debugging.",
        usage: &["chic mir-dump <file>"],
        options: &[
            OptionGuide {
                flag: "--consteval-fuel <n>",
                description: "Override the const-eval fuel limit for this invocation.",
            },
            OptionGuide {
                flag: "--log-format <format>",
                description: "Select log output format (auto, text, json).",
            },
            OptionGuide {
                flag: "--log-level <level>",
                description: "Set log verbosity (error, warn, info, debug, trace).",
            },
            OptionGuide {
                flag: "--error-format <format>",
                description: "Select diagnostic format (human, json, toon, short); defaults depend on TTY.",
            },
            OptionGuide {
                flag: "--trace-pipeline",
                description: "Emit structured tracing spans for pipeline stages.",
            },
            OptionGuide {
                flag: "--trait-solver-metrics",
                description: "Enable trait solver telemetry (logs + CLI summary).",
            },
        ],
        examples: &["chic mir-dump examples/hello.ch"],
        docs: &["docs/mir_design.md", "docs/guides/logging.md"],
    },
    CommandGuide {
        names: &["header"],
        summary: "Generate a C-compatible header for public APIs.",
        usage: &["chic header <file> [options]"],
        options: &[
            OptionGuide {
                flag: "-o, --output <path>",
                description: "Write the generated header to <path>.",
            },
            OptionGuide {
                flag: "--include-guard <IDENT>",
                description: "Override the include guard identifier.",
            },
            OptionGuide {
                flag: "--error-format <format>",
                description: "Select diagnostic format (human, json, toon, short); defaults depend on TTY.",
            },
        ],
        examples: &[
            "chic header api.ch",
            "chic header api.ch -o include/api.h --include-guard API_H",
        ],
        docs: &["docs/header_generation.md"],
    },
    CommandGuide {
        names: &["extern bind"],
        summary: "Generate Chic `@extern` wrappers from a C header.",
        usage: &[
            "chic extern bind --library <name> --header <path> --namespace <ns> --output <file> [options]",
        ],
        options: &[
            OptionGuide {
                flag: "--library <name>",
                description: "Symbol prefix or dylib stem to use for generated bindings.",
            },
            OptionGuide {
                flag: "--header <path>",
                description: "C header containing prototypes to convert.",
            },
            OptionGuide {
                flag: "--namespace <name>",
                description: "Namespace that will contain the generated functions.",
            },
            OptionGuide {
                flag: "--output <path>",
                description: "Destination Chic source file (directories created as needed).",
            },
            OptionGuide {
                flag: "--binding <mode>",
                description: "Override the generated binding (lazy, eager, static).",
            },
            OptionGuide {
                flag: "--convention <abi>",
                description: "Override the calling convention (defaults to `system`).",
            },
            OptionGuide {
                flag: "--optional",
                description: "Mark generated bindings as optional (missing libs return zero).",
            },
        ],
        examples: &[
            "chic extern bind --library sqlite3 --header /usr/include/sqlite3.h --namespace Std.Interop.Sqlite --output packages/std/src/sqlite.ch",
        ],
        docs: &["docs/runtime/dynamic_ffi.md"],
    },
    CommandGuide {
        names: &["perf report", "perf", "perf-report"],
        summary: "Summarise perf.json runs and compare against a baseline.",
        usage: &[
            "chic perf [perf.json] [options]",
            "chic perf-report [perf.json] [options]",
        ],
        options: &[
            OptionGuide {
                flag: "-b, --baseline <path>",
                description: "Compare against a baseline perf.json file.",
            },
            OptionGuide {
                flag: "-p, --profile <name>",
                description: "Select a profile name when multiple runs exist (e.g. debug/release).",
            },
            OptionGuide {
                flag: "--tolerance <percent>",
                description: "Allowed regression percentage before failing (default: 5).",
            },
            OptionGuide {
                flag: "--fail-on-regressions",
                description: "Exit non-zero when regressions exceed --tolerance.",
            },
            OptionGuide {
                flag: "-j, --json",
                description: "Emit machine-readable JSON output.",
            },
        ],
        examples: &[
            "chic perf profiling/latest/perf.json",
            "chic perf profiling/latest/perf.json --baseline main/perf.json --fail-on-regressions",
        ],
        docs: &["docs/tooling/perf_json.md"],
    },
    CommandGuide {
        names: &["seed"],
        summary: "Extract RNG seeds from perf/run logs for deterministic replay.",
        usage: &["chic seed --from-run <perf.json|runlog> [--profile <name>] [--json]"],
        options: &[
            OptionGuide {
                flag: "--from-run <path>",
                description: "Path to perf.json or runlog containing RNG events (defaults to perf.json).",
            },
            OptionGuide {
                flag: "--profile <name>",
                description: "Select a profile when multiple perf runs are present.",
            },
            OptionGuide {
                flag: "--json, -j",
                description: "Emit machine-readable JSON instead of text summary.",
            },
        ],
        examples: &[
            "chic seed --from-run profiling/latest/perf.json --profile debug",
            "chic seed runlog.json --json",
        ],
        docs: &["docs/tooling/perf_json.md", "docs/runtime/random.md"],
    },
    CommandGuide {
        names: &["spec", "show-spec"],
        summary: "Print the language specification location and summary.",
        usage: &["chic spec"],
        options: &[],
        examples: &["chic spec"],
        docs: &["SPEC.md"],
    },
    CommandGuide {
        names: &["version", "--version", "-V"],
        summary: "Display Chic version, commit hash, and build metadata.",
        usage: &["chic --version", "chic version"],
        options: &[],
        examples: &["chic --version"],
        docs: &["README.md#Getting-Started"],
    },
];

pub(crate) fn render_general_help() -> String {
    let mut out = String::new();
    out.push_str("Chic – Chic bootstrap toolchain\n");
    out.push_str("manifest.yaml is the Chic project file and controls builds.\n\n");
    out.push_str("USAGE:\n  chic <command> [options]\n\n");
    out.push_str("PROJECT RESOLUTION:\n");
    out.push_str("  chic build              # uses manifest.yaml in the current directory\n");
    out.push_str("  chic build manifest.yaml | path/to/dir | path/to/manifest.yaml\n");
    out.push_str("  chic run/test accept the same project-or-directory argument style.\n\n");
    out.push_str("COMMANDS:\n");
    for guide in COMMAND_GUIDES {
        let canonical = guide.names[0];
        let _ = writeln!(out, "  {:11} {}", canonical, guide.summary);
    }
    out.push('\n');
    out.push_str("GLOBAL OPTIONS:\n");
    for option in GLOBAL_OPTIONS {
        let _ = writeln!(out, "  {:18} {}", option.flag, option.description);
    }
    out.push('\n');
    out.push_str("DOCS:\n");
    for doc in GENERAL_DOCS {
        let _ = writeln!(out, "  {doc}");
    }
    out.push('\n');
    out.push_str("Use `chic help <command>` to view detailed usage and examples.");
    out.push('\n');
    out
}

pub(crate) fn render_command_help(topic: &str) -> Option<String> {
    let guide = find_guide(topic)?;
    let mut out = String::new();
    let canonical = guide.names[0];
    let _ = writeln!(out, "Chic {canonical} – {}", guide.summary);
    out.push('\n');

    out.push_str("USAGE:\n");
    for usage in guide.usage {
        let _ = writeln!(out, "  {usage}");
    }

    if guide.names.len() > 1 {
        out.push('\n');
        out.push_str("ALIASES:\n");
        for alias in &guide.names[1..] {
            let _ = writeln!(out, "  {alias}");
        }
    }

    if !guide.options.is_empty() {
        out.push('\n');
        out.push_str("OPTIONS:\n");
        for option in guide.options {
            let _ = writeln!(out, "  {:24} {}", option.flag, option.description);
        }
    }

    if !guide.examples.is_empty() {
        out.push('\n');
        out.push_str("EXAMPLES:\n");
        for example in guide.examples {
            let _ = writeln!(out, "  {example}");
        }
    }

    if !guide.docs.is_empty() {
        out.push('\n');
        out.push_str("DOCS:\n");
        for doc in guide.docs {
            let _ = writeln!(out, "  {doc}");
        }
    }

    out.push('\n');
    out.push_str("All commands accept `-h`/`--help` for contextual guidance.");
    out.push('\n');
    Some(out)
}

pub(crate) fn available_topics() -> impl Iterator<Item = &'static str> {
    COMMAND_GUIDES.iter().map(|guide| guide.names[0])
}

pub(crate) fn format_unknown_topic(topic: &str) -> String {
    let mut known = available_topics().collect::<Vec<_>>();
    known.sort_unstable();
    format!(
        "unknown help topic '{topic}'; available commands: {}",
        known.join(", ")
    )
}

fn find_guide(topic: &str) -> Option<&'static CommandGuide> {
    let lower = topic.to_ascii_lowercase();
    COMMAND_GUIDES.iter().find(|guide| {
        guide
            .names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(&lower))
    })
}
