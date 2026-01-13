use super::*;
use super::{CommandRegistry, commands};
use crate::chic_kind::ChicKind;
use crate::codegen::CpuIsaTier;
use crate::diagnostics::ErrorFormat;
use crate::logging::{LogFormat, LogLevel};
use crate::runtime::backend::RuntimeBackend;
use crate::target::TargetArch;
use once_cell::sync::Lazy;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::tempdir;

static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn with_env_lock<F>(f: F)
where
    F: FnOnce(),
{
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f();
}

fn with_locked_env<F>(f: F)
where
    F: FnOnce(),
{
    with_env_lock(f);
}

fn set_env_var(key: &str, value: &str) {
    unsafe { env::set_var(key, value) };
}

fn remove_env_var(key: &str) {
    unsafe { env::remove_var(key) };
}

fn expect_cli_ok<I, T>(args: I) -> Cli
where
    I: IntoIterator<Item = T>,
    T: Into<String>,
{
    match Cli::parse_from(args.into_iter()) {
        Ok(cli) => cli,
        Err(err) => panic!("expected CLI parse to succeed, found error: {err}"),
    }
}

#[test]
fn registry_resolves_canonical_and_alias_commands() {
    let registry = registry();
    let build = registry
        .resolve("build")
        .expect("build command must be registered");
    assert_eq!(build.name(), "build");
    assert!(
        registry.resolve("spec").is_some(),
        "spec must be registered"
    );
    let alias = registry
        .resolve("show-spec")
        .expect("show-spec alias must be registered");
    assert_eq!(alias.name(), "spec");
    assert!(
        registry.resolve("nonexistent").is_none(),
        "unexpected registration for unknown command"
    );
    let aliases: Vec<_> = registry
        .iter()
        .flat_map(|descriptor| descriptor.aliases().iter().copied())
        .collect();
    assert!(
        aliases.contains(&"show-spec"),
        "show-spec alias should be reported via descriptor aliases"
    );
}

#[test]
fn registry_hides_feature_flagged_commands_when_disabled() {
    let registry = CommandRegistry::new(commands::descriptors(), |_| false);
    let available: Vec<_> = registry
        .iter()
        .map(|descriptor| descriptor.name())
        .collect();
    assert!(registry.resolve("cc1").is_none(), "cc1 must be gated off");
    assert!(
        registry.resolve("extern").is_none(),
        "extern bind must be gated off"
    );
    assert!(
        registry.resolve("build").is_some(),
        "non-gated commands remain available"
    );
    assert!(
        !available.contains(&"cc1") && !available.contains(&"extern"),
        "feature-gated commands must be omitted from iteration"
    );
}

#[test]
fn registry_respects_environment_opt_out() {
    with_env_lock(|| {
        set_env_var("CHIC_DISABLE_CLI_FEATURES", "1");
        let registry = registry();
        assert!(
            registry.resolve("cc1").is_none() && registry.resolve("extern").is_none(),
            "cc1 and extern bind must be hidden when CHIC_DISABLE_CLI_FEATURES=1"
        );
        let available: Vec<_> = registry
            .iter()
            .map(|descriptor| descriptor.name())
            .collect();
        assert!(
            !available.contains(&"cc1") && !available.contains(&"extern"),
            "feature-disabled commands must not appear in the registry when opt-out is set"
        );
        remove_env_var("CHIC_DISABLE_CLI_FEATURES");
    });
}

#[test]
fn registry_construction_calls_feature_filter() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    fn tracking(_feature: CommandFeature) -> bool {
        CALLS.fetch_add(1, Ordering::SeqCst);
        true
    }
    let registry = CommandRegistry::new(commands::descriptors(), tracking);
    assert!(
        registry.iter().count() >= 1,
        "registry must expose commands"
    );
    assert!(
        CALLS.load(Ordering::SeqCst) > 0,
        "feature filter should be invoked during iteration"
    );
}

#[test]
fn cli_error_display_round_trips_message() {
    let err = CliError::new("oops");
    assert_eq!(err.to_string(), "oops");
}

#[test]
fn cli_ffi_options_default_is_empty() {
    let defaults = CliFfiOptions::default();
    assert!(defaults.search_paths.is_empty());
    assert!(defaults.default_patterns.is_empty());
    assert!(defaults.package_globs.is_empty());
}

fn expect_cli_err<I, T>(args: I) -> CliError
where
    I: IntoIterator<Item = T>,
    T: Into<String>,
{
    match Cli::parse_from(args.into_iter()) {
        Ok(cli) => panic!("expected CLI parse to fail, found command: {cli:?}"),
        Err(err) => err,
    }
}

#[test]
fn parses_check_command() {
    let cli = expect_cli_ok(["check", "main.cl"]);
    match cli.command {
        Command::Check {
            inputs,
            kind,
            trace_pipeline,
            ..
        } => {
            assert_eq!(inputs, vec![PathBuf::from("main.cl")]);
            assert_eq!(kind, ChicKind::Executable);
            assert!(!trace_pipeline);
        }
        other => panic!("expected check command, found {other:?}"),
    }
}

#[test]
fn check_accepts_trace_pipeline_flag() {
    let cli = expect_cli_ok(["check", "main.cl", "--trace-pipeline"]);
    match cli.command {
        Command::Check { trace_pipeline, .. } => assert!(trace_pipeline),
        other => panic!("expected check command, found {other:?}"),
    }
}

#[test]
fn check_accepts_trait_solver_metrics_flag() {
    let cli = expect_cli_ok(["check", "main.cl", "--trait-solver-metrics"]);
    match cli.command {
        Command::Check {
            trait_solver_metrics,
            ..
        } => assert!(trait_solver_metrics),
        other => panic!("expected check command, found {other:?}"),
    }
}

#[test]
fn check_accepts_log_flags() {
    let cli = expect_cli_ok([
        "check",
        "main.cl",
        "--log-format",
        "json",
        "--log-level",
        "debug",
    ]);
    assert_eq!(cli.log_options.format, LogFormat::Json);
    assert_eq!(cli.log_options.level, LogLevel::Debug);
}

#[test]
fn check_accepts_error_format_flag() {
    let cli = expect_cli_ok(["check", "main.cl", "--error-format", "json"]);
    assert_eq!(cli.error_format, Some(ErrorFormat::Json));
}

#[test]
fn check_rejects_unknown_error_format() {
    let err = expect_cli_err(["check", "main.cl", "--error-format", "bogus"]);
    assert!(
        err.to_string().contains("invalid --error-format 'bogus'"),
        "error message should mention invalid format: {err}"
    );
}

#[test]
fn parses_global_help_flag() {
    let cli = expect_cli_ok(["--help"]);
    match cli.command {
        Command::Help { ref topic } => assert!(topic.is_none()),
        other => panic!("expected help command, found {other:?}"),
    }
}

#[test]
fn parses_global_help_with_topic_and_lowercases() {
    let cli = expect_cli_ok(["--help", "BUILD"]);
    match cli.command {
        Command::Help { ref topic } => assert_eq!(topic.as_deref(), Some("build")),
        other => panic!("expected help command, found {other:?}"),
    }
}

#[test]
fn parses_global_help_double_help_flag_as_plain_help() {
    let cli = expect_cli_ok(["--help", "--help"]);
    match cli.command {
        Command::Help { ref topic } => assert!(topic.is_none()),
        other => panic!("expected help command, found {other:?}"),
    }
}

#[test]
fn parses_help_command_with_topic() {
    let cli = expect_cli_ok(["help", "build"]);
    match cli.command {
        Command::Help { ref topic } => {
            assert_eq!(topic.as_deref(), Some("build"));
        }
        other => panic!("expected help command, found {other:?}"),
    }
}

#[test]
fn parses_subcommand_help_flag() {
    let cli = expect_cli_ok(["build", "--help"]);
    match cli.command {
        Command::Help { ref topic } => {
            assert_eq!(topic.as_deref(), Some("build"));
        }
        other => panic!("expected help command, found {other:?}"),
    }
}

#[test]
fn parses_help_after_input() {
    let cli = expect_cli_ok(["build", "main.cl", "--help"]);
    match cli.command {
        Command::Help { ref topic } => {
            assert_eq!(topic.as_deref(), Some("build"));
        }
        other => panic!("expected help command, found {other:?}"),
    }
}

#[test]
fn parses_extern_bind_command() {
    let cli = expect_cli_ok([
        "extern",
        "bind",
        "--library",
        "sample",
        "--header",
        "sample.h",
        "--namespace",
        "Tests.Interop",
        "--output",
        "bindings.cl",
    ]);
    match cli.command {
        Command::ExternBind {
            library,
            namespace,
            header,
            output,
            ..
        } => {
            assert_eq!(library, "sample");
            assert_eq!(namespace, "Tests.Interop");
            assert_eq!(header, PathBuf::from("sample.h"));
            assert_eq!(output, PathBuf::from("bindings.cl"));
        }
        other => panic!("expected extern bind command, found {other:?}"),
    }
}

#[test]
fn general_help_matches_snapshot() {
    let help = Cli::usage();
    let snapshot = include_str!("../../tests/snapshots/help_general.txt");
    assert_eq!(help, snapshot);
}

#[test]
fn init_help_matches_snapshot() {
    let help = Cli::help_for("init").expect("init help must render");
    let snapshot = include_str!("../../tests/snapshots/help_init.txt");
    assert_eq!(help, snapshot);
}

#[test]
fn build_help_matches_snapshot() {
    let help = Cli::help_for("build").expect("build help must render");
    let snapshot = include_str!("../../tests/snapshots/help_build.txt");
    assert_eq!(help, snapshot);
}

#[test]
fn run_help_matches_snapshot() {
    let help = Cli::help_for("run").expect("run help must render");
    let snapshot = include_str!("../../tests/snapshots/help_run.txt");
    assert_eq!(help, snapshot);
}

#[test]
fn test_help_matches_snapshot() {
    let help = Cli::help_for("test").expect("test help must render");
    let snapshot = include_str!("../../tests/snapshots/help_test.txt");
    assert_eq!(help, snapshot);
}

#[test]
fn log_options_default_from_environment() {
    with_env_lock(|| {
        let prev_format = env::var("CHIC_LOG_FORMAT").ok();
        let prev_level = env::var("CHIC_LOG_LEVEL").ok();
        set_env_var("CHIC_LOG_FORMAT", "json");
        set_env_var("CHIC_LOG_LEVEL", "trace");
        let cli = expect_cli_ok(["format", "example.cl"]);
        assert_eq!(cli.log_options.format, LogFormat::Json);
        assert_eq!(cli.log_options.level, LogLevel::Trace);
        if let Some(value) = prev_format {
            set_env_var("CHIC_LOG_FORMAT", value.as_str());
        } else {
            remove_env_var("CHIC_LOG_FORMAT");
        }
        if let Some(value) = prev_level {
            set_env_var("CHIC_LOG_LEVEL", value.as_str());
        } else {
            remove_env_var("CHIC_LOG_LEVEL");
        }
    });
}

fn make_project(include_tests: bool) -> (tempfile::TempDir, PathBuf, Option<PathBuf>) {
    let dir = tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join("src")).expect("create src");
    fs::write(
        dir.path().join(crate::manifest::PROJECT_MANIFEST_BASENAME),
        "package:\n  name: sample\nbuild:\n  configuration: Debug\n",
    )
    .expect("write manifest");
    let main = dir.path().join("src/main.cl");
    fs::write(&main, "namespace Sample;").expect("write main");
    let tests = if include_tests {
        let test_path = dir.path().join("tests/math.cl");
        fs::create_dir_all(test_path.parent().unwrap()).expect("create tests dir");
        fs::write(&test_path, "namespace Sample.Tests; testcase Math;").expect("write test");
        Some(test_path)
    } else {
        None
    };
    (dir, main, tests)
}

#[test]
fn build_discovers_manifest_in_cwd() {
    with_locked_env(|| {
        let prev = env::current_dir().expect("cwd");
        let (project, main, _) = make_project(false);
        env::set_current_dir(project.path()).expect("set cwd");
        let cli = expect_cli_ok(["build"]);
        match cli.command {
            Command::Build { inputs, .. } => {
                let expected = fs::canonicalize(&main).unwrap_or(main.clone());
                let actual: Vec<_> = inputs
                    .iter()
                    .map(|path| fs::canonicalize(path).unwrap_or(path.clone()))
                    .collect();
                assert_eq!(actual, vec![expected]);
            }
            other => panic!("expected build command, found {other:?}"),
        }
        env::set_current_dir(prev).expect("restore cwd");
    });
}

#[test]
fn build_accepts_explicit_manifest_path() {
    with_locked_env(|| {
        let (project, main, _) = make_project(false);
        let manifest = project
            .path()
            .join(crate::manifest::PROJECT_MANIFEST_BASENAME);
        let cli = expect_cli_ok([
            "build",
            manifest
                .to_str()
                .expect("manifest path must be valid UTF-8"),
        ]);
        match cli.command {
            Command::Build { inputs, .. } => {
                let expected = fs::canonicalize(&main).unwrap_or(main.clone());
                let actual: Vec<_> = inputs
                    .iter()
                    .map(|path| fs::canonicalize(path).unwrap_or(path.clone()))
                    .collect();
                assert_eq!(actual, vec![expected]);
            }
            other => panic!("expected build command, found {other:?}"),
        }
    });
}

#[test]
fn build_accepts_directory_argument() {
    with_locked_env(|| {
        let (project, main, _) = make_project(false);
        let cli = expect_cli_ok([
            "build",
            project
                .path()
                .to_str()
                .expect("project path must be valid UTF-8"),
        ]);
        match cli.command {
            Command::Build { inputs, .. } => {
                let expected = fs::canonicalize(&main).unwrap_or(main.clone());
                let actual: Vec<_> = inputs
                    .iter()
                    .map(|path| fs::canonicalize(path).unwrap_or(path.clone()))
                    .collect();
                assert_eq!(actual, vec![expected]);
            }
            other => panic!("expected build command, found {other:?}"),
        }
    });
}

#[test]
fn build_defaults_to_manifest_library_kind() {
    with_locked_env(|| {
        let dir = tempdir().expect("tempdir");
        fs::create_dir_all(dir.path().join("src")).expect("create src");
        let manifest_path = dir.path().join(crate::manifest::PROJECT_MANIFEST_BASENAME);
        fs::write(
            &manifest_path,
            "package:\n  name: sample\nbuild:\n  kind: lib\n",
        )
        .expect("write manifest");
        let main = dir.path().join("src/main.cl");
        fs::write(&main, "namespace Sample;").expect("write main");

        let cli = expect_cli_ok([
            "build",
            manifest_path
                .to_str()
                .expect("manifest path must be valid UTF-8"),
        ]);
        match cli.command {
            Command::Build { kind, .. } => assert_eq!(kind, ChicKind::StaticLibrary),
            other => panic!("expected build command, found {other:?}"),
        }
    });
}

#[test]
fn build_kind_cli_override_wins_over_manifest() {
    with_locked_env(|| {
        let dir = tempdir().expect("tempdir");
        fs::create_dir_all(dir.path().join("src")).expect("create src");
        let manifest_path = dir.path().join(crate::manifest::PROJECT_MANIFEST_BASENAME);
        fs::write(
            &manifest_path,
            "package:\n  name: sample\nbuild:\n  kind: lib\n",
        )
        .expect("write manifest");
        let main = dir.path().join("src/main.cl");
        fs::write(&main, "namespace Sample;").expect("write main");

        let cli = expect_cli_ok([
            "build",
            manifest_path
                .to_str()
                .expect("manifest path must be valid UTF-8"),
            "--crate-type",
            "exe",
        ]);
        match cli.command {
            Command::Build { kind, .. } => assert_eq!(kind, ChicKind::Executable),
            other => panic!("expected build command, found {other:?}"),
        }
    });
}

#[test]
fn run_and_test_resolve_project_sources() {
    with_locked_env(|| {
        let (project, main, maybe_test) = make_project(true);
        let project_path = project
            .path()
            .to_str()
            .expect("project path must be valid UTF-8");

        let run_cli = expect_cli_ok(["run", project_path]);
        match run_cli.command {
            Command::Run { inputs, .. } => {
                let expected_main = fs::canonicalize(&main).unwrap_or(main.clone());
                let actual: Vec<_> = inputs
                    .iter()
                    .map(|path| fs::canonicalize(path).unwrap_or(path.clone()))
                    .collect();
                assert_eq!(actual, vec![expected_main]);
            }
            other => panic!("expected run command, found {other:?}"),
        }

        let test_cli = expect_cli_ok(["test", project_path]);
        match test_cli.command {
            Command::Test { inputs, .. } => {
                let mut expected = vec![fs::canonicalize(&main).unwrap_or(main.clone())];
                if let Some(test) = maybe_test {
                    expected.push(fs::canonicalize(&test).unwrap_or(test));
                }
                expected.sort();
                let mut actual: Vec<_> = inputs
                    .iter()
                    .map(|path| fs::canonicalize(path).unwrap_or(path.clone()))
                    .collect();
                actual.sort();
                assert_eq!(actual, expected);
            }
            other => panic!("expected test command, found {other:?}"),
        }
    });
}

#[test]
fn configuration_layers_project_workspace_env_and_cli() {
    with_locked_env(|| {
        let prev = env::current_dir().expect("cwd");
        let prev_config = env::var("CHIC_CONFIGURATION").ok();
        let root = tempdir().expect("workspace root");
        let project_dir = root.path().join("app");
        fs::create_dir_all(project_dir.join("src")).expect("create src");
        fs::write(project_dir.join("src/main.cl"), "namespace Sample;").expect("write main");
        fs::write(
            project_dir.join(crate::manifest::PROJECT_MANIFEST_BASENAME),
            "build:\n  configuration: ManifestConfig\n",
        )
        .expect("write manifest");
        fs::write(
            root.path()
                .join(crate::manifest::WORKSPACE_MANIFEST_BASENAME),
            "build:\n  configuration: WorkspaceConfig\n",
        )
        .expect("write workspace");
        env::set_current_dir(&project_dir).expect("set cwd");

        set_env_var("CHIC_CONFIGURATION", "EnvConfig");
        let cli = expect_cli_ok(["build", "--configuration", "CliConfig"]);
        match cli.command {
            Command::Build { configuration, .. } => assert_eq!(configuration, "CliConfig"),
            other => panic!("expected build command, found {other:?}"),
        }

        let cli_env = expect_cli_ok(["build"]);
        match cli_env.command {
            Command::Build { configuration, .. } => assert_eq!(configuration, "EnvConfig"),
            other => panic!("expected build command, found {other:?}"),
        }

        remove_env_var("CHIC_CONFIGURATION");
        let cli_workspace = expect_cli_ok(["build"]);
        match cli_workspace.command {
            Command::Build { configuration, .. } => assert_eq!(configuration, "WorkspaceConfig"),
            other => panic!("expected build command, found {other:?}"),
        }

        match prev_config {
            Some(value) => set_env_var("CHIC_CONFIGURATION", value.as_str()),
            None => remove_env_var("CHIC_CONFIGURATION"),
        }
        env::set_current_dir(prev).expect("restore cwd");
    });
}

#[test]
fn missing_project_reports_clear_error() {
    with_locked_env(|| {
        let prev = env::current_dir().expect("cwd");
        let dir = tempdir().expect("dir");
        env::set_current_dir(dir.path()).expect("set cwd");
        let err = expect_cli_err(["build"]);
        assert!(
            err.to_string()
                .contains("No manifest.yaml project file found"),
            "unexpected message: {err}"
        );
        env::set_current_dir(prev).expect("restore cwd");
    });
}

#[test]
fn help_for_unknown_topic_reports_error() {
    let err = Cli::help_for("unknown-command").expect_err("expected help lookup failure");
    assert!(
        err.to_string()
            .contains("unknown help topic 'unknown-command'"),
        "unexpected error: {err}"
    );
}

#[test]
fn parses_build_default_backend() {
    with_locked_env(|| {
        let prev = env::current_dir().expect("cwd");
        env::set_current_dir(env!("CARGO_MANIFEST_DIR")).expect("set cwd");

        let cli = expect_cli_ok(["build", "main.cl"]);
        match cli.command {
            Command::Build {
                inputs,
                backend,
                kind,
                emit_wat,
                runtime_backend,
                ..
            } => {
                let expected = env::current_dir().expect("cwd").join("main.cl");
                assert_eq!(inputs, vec![expected]);
                assert_eq!(backend, Backend::Llvm);
                assert_eq!(kind, ChicKind::Executable);
                assert!(!emit_wat);
                assert_eq!(
                    runtime_backend,
                    crate::runtime::backend::RuntimeBackend::Chic
                );
            }
            other => panic!("expected build command, found {other:?}"),
        }

        env::set_current_dir(prev).expect("restore cwd");
    });
}

#[test]
fn parses_chic_runtime_flag_for_build() {
    with_locked_env(|| {
        let cli = expect_cli_ok(["build", "main.cl", "--runtime-backend", "chic"]);
        match cli.command {
            Command::Build {
                runtime_backend, ..
            } => assert_eq!(runtime_backend, RuntimeBackend::Chic),
            other => panic!("expected build command, found {other:?}"),
        }
    });
}

#[test]
fn rejects_rust_runtime_flag() {
    with_locked_env(|| {
        let err = expect_cli_err(["build", "main.cl", "--runtime-backend", "rust"]);
        assert!(
            err.to_string().contains("runtime shim has been removed")
                || err.to_string().contains("only supported runtime"),
            "missing rust runtime removal message: {err}"
        );
    });
}

#[test]
fn parses_build_with_target_and_output() {
    with_locked_env(|| {
        let prev = env::current_dir().expect("cwd");
        env::set_current_dir(env!("CARGO_MANIFEST_DIR")).expect("set cwd");
        let cli = expect_cli_ok([
            "build",
            "main.cl",
            "--target",
            "x86_64-unknown-linux-gnu",
            "-o",
            "out.clbin",
            "--backend",
            "llvm",
        ]);
        match cli.command {
            Command::Build {
                inputs,
                target,
                output,
                backend,
                kind,
                emit_wat,
                ..
            } => {
                let expected = env::current_dir().expect("cwd").join("main.cl");
                assert_eq!(inputs, vec![expected]);
                assert_eq!(target.triple(), "x86_64-unknown-linux-gnu");
                assert_eq!(output.as_deref(), Some(Path::new("out.clbin")));
                assert_eq!(backend, Backend::Llvm);
                assert_eq!(kind, ChicKind::Executable);
                assert!(!emit_wat);
            }
            other => panic!("expected build command, found {other:?}"),
        }
        env::set_current_dir(prev).expect("restore cwd");
    });
}

#[test]
fn build_accepts_emit_header_for_library() {
    let cli = expect_cli_ok(["build", "lib.cl", "--crate-type", "lib", "--emit-header"]);
    match cli.command {
        Command::Build {
            emit_header,
            emit_lib,
            kind,
            ..
        } => {
            assert!(emit_header, "expected --emit-header to set emit_header");
            assert!(!emit_lib, "--emit-lib was not specified");
            assert!(kind.is_library(), "expected library crate type");
        }
        other => panic!("expected build command, found {other:?}"),
    }
}

#[test]
fn build_accepts_trace_pipeline_flag() {
    let cli = expect_cli_ok(["build", "main.cl", "--trace-pipeline"]);
    match cli.command {
        Command::Build { trace_pipeline, .. } => assert!(trace_pipeline),
        other => panic!("expected build command, found {other:?}"),
    }
}

#[test]
fn build_accepts_trait_solver_metrics_flag() {
    let cli = expect_cli_ok(["build", "main.cl", "--trait-solver-metrics"]);
    match cli.command {
        Command::Build {
            trait_solver_metrics,
            ..
        } => assert!(trait_solver_metrics),
        other => panic!("expected build command, found {other:?}"),
    }
}

#[test]
fn run_accepts_trace_pipeline_flag() {
    let cli = expect_cli_ok(["run", "main.cl", "--trace-pipeline"]);
    match cli.command {
        Command::Run { trace_pipeline, .. } => assert!(trace_pipeline),
        other => panic!("expected run command, found {other:?}"),
    }
}

#[test]
fn run_enables_profile_when_requested() {
    let cli = expect_cli_ok(["run", "main.cl", "--profile", "--profile-sample-ms", "5"]);
    match cli.command {
        Command::Run { profile, .. } => {
            let opts = profile.expect("profile should be enabled");
            assert_eq!(opts.sample_ms, Some(5));
            assert_eq!(opts.output, PathBuf::from("profiling/latest/perf.json"));
            assert!(!opts.flamegraph);
        }
        other => panic!("expected run command, found {other:?}"),
    }
}

#[test]
fn profile_command_defaults_to_flamegraph() {
    let cli = expect_cli_ok(["profile", "main.cl"]);
    match cli.command {
        Command::Run { profile, .. } => {
            let opts = profile.expect("profile command should enable profiling");
            assert!(opts.flamegraph);
            assert_eq!(opts.output, PathBuf::from("profiling/latest/perf.json"));
        }
        other => panic!("expected run command, found {other:?}"),
    }
}

#[test]
fn test_accepts_trace_pipeline_flag() {
    let cli = expect_cli_ok(["test", "suite.cl", "--trace-pipeline"]);
    match cli.command {
        Command::Test { trace_pipeline, .. } => assert!(trace_pipeline),
        other => panic!("expected test command, found {other:?}"),
    }
}

#[test]
fn mir_dump_accepts_trace_pipeline_flag() {
    let cli = expect_cli_ok(["mir-dump", "main.cl", "--trace-pipeline"]);
    match cli.command {
        Command::MirDump { trace_pipeline, .. } => assert!(trace_pipeline),
        other => panic!("expected mir-dump command, found {other:?}"),
    }
}

#[test]
fn mir_dump_accepts_trait_solver_metrics_flag() {
    let cli = expect_cli_ok(["mir-dump", "main.cl", "--trait-solver-metrics"]);
    match cli.command {
        Command::MirDump {
            trait_solver_metrics,
            ..
        } => assert!(trait_solver_metrics),
        other => panic!("expected mir-dump command, found {other:?}"),
    }
}

#[test]
fn build_rejects_emit_header_for_executable() {
    let err = expect_cli_err(["build", "main.cl", "--emit-header"]);
    assert!(
        err.to_string()
            .contains("--emit-header/--emit-lib require a library crate type")
    );
}

#[test]
fn build_rejects_emit_header_for_run_command() {
    let err = expect_cli_err(["run", "main.cl", "--emit-header"]);
    assert!(
        err.to_string()
            .contains("--emit-header is only supported for chic build")
    );
}

#[test]
fn build_accepts_emit_lib_for_library() {
    let cli = expect_cli_ok(["build", "lib.cl", "--crate-type", "lib", "--emit-lib"]);
    match cli.command {
        Command::Build {
            emit_header,
            emit_lib,
            backend,
            ..
        } => {
            assert!(!emit_header, "--emit-header not provided");
            assert!(emit_lib, "expected emit_lib to be set");
            assert_eq!(backend, Backend::Llvm);
        }
        other => panic!("expected build command, found {other:?}"),
    }
}

#[test]
fn build_rejects_emit_lib_for_non_library() {
    let err = expect_cli_err(["build", "main.cl", "--emit-lib"]);
    assert!(
        err.to_string()
            .contains("--emit-header/--emit-lib require a library crate type")
    );
}

#[test]
fn build_rejects_emit_lib_on_wasm_backend() {
    let err = expect_cli_err([
        "build",
        "lib.cl",
        "--crate-type",
        "lib",
        "--backend",
        "wasm",
        "--emit-lib",
    ]);
    assert!(
        err.to_string()
            .contains("--emit-lib currently requires the LLVM backend")
    );
}

#[test]
fn run_rejects_emit_lib_option() {
    let err = expect_cli_err(["run", "lib.cl", "--emit-lib"]);
    assert!(
        err.to_string()
            .contains("--emit-lib is only supported for chic build")
    );
}

#[test]
fn build_accepts_cc1_backend_with_args() {
    let cli = expect_cli_ok([
        "build",
        "module.cl",
        "--backend",
        "cc1",
        "--cc1-arg",
        "-debug-info-kind=line-tables-only",
        "--cc1-keep-input",
    ]);
    match cli.command {
        Command::Build {
            backend,
            cc1_args,
            cc1_keep_temps,
            ..
        } => {
            assert_eq!(backend, Backend::Cc1);
            assert!(cc1_args.contains(&"-debug-info-kind=line-tables-only".to_string()));
            assert!(cc1_keep_temps);
        }
        other => panic!("expected build command, found {other:?}"),
    }
}

#[test]
fn parses_cc1_command_with_defaults() {
    let cli = expect_cli_ok(["cc1", "input.i"]);
    match cli.command {
        Command::Cc1 {
            ref input,
            ref output,
            extra_args,
            ..
        } => {
            assert_eq!(input, &PathBuf::from("input.i"));
            assert!(output.is_none());
            assert!(extra_args.is_empty());
        }
        other => panic!("expected cc1 command, found {other:?}"),
    }
}

#[test]
fn parses_cc1_with_output_and_args() {
    let cli = expect_cli_ok([
        "cc1",
        "in.i",
        "-o",
        "out.s",
        "--cc1-arg",
        "-debug-info-kind=limited",
        "--target",
        "x86_64-unknown-linux-gnu",
    ]);
    match cli.command {
        Command::Cc1 {
            ref input,
            ref output,
            ref extra_args,
            ref target,
        } => {
            assert_eq!(input, &PathBuf::from("in.i"));
            assert_eq!(output.as_deref(), Some(Path::new("out.s")));
            assert!(extra_args.contains(&"-debug-info-kind=limited".to_string()));
            assert_eq!(target.triple(), "x86_64-unknown-linux-gnu");
        }
        other => panic!("expected cc1 command, found {other:?}"),
    }
}

#[test]
fn run_rejects_library_kind() {
    let err = expect_cli_err(["run", "main.cl", "--crate-type", "lib"]);
    assert!(
        err.to_string()
            .contains("chic run only supports executable crate types")
    );
}

#[test]
fn parses_version_flag() {
    let cli = expect_cli_ok(["--version"]);
    assert!(matches!(cli.command, Command::Version));
}

#[test]
fn parses_version_command() {
    let cli = expect_cli_ok(["version"]);
    assert!(matches!(cli.command, Command::Version));
}

#[test]
fn parses_version_help() {
    let cli = expect_cli_ok(["version", "--help"]);
    match cli.command {
        Command::Help { ref topic } => assert_eq!(topic.as_deref(), Some("version")),
        other => panic!("expected help command, found {other:?}"),
    }
}

#[test]
fn parses_version_flag_help() {
    let cli = expect_cli_ok(["--version", "--help"]);
    match cli.command {
        Command::Help { ref topic } => assert_eq!(topic.as_deref(), Some("version")),
        other => panic!("expected help command, found {other:?}"),
    }
}

#[test]
fn version_flag_rejects_additional_arguments() {
    let err = expect_cli_err(["--version", "extra"]);
    assert!(
        err.to_string()
            .contains("unsupported option 'extra' for command")
    );
}

#[test]
fn version_command_rejects_additional_arguments() {
    let err = expect_cli_err(["version", "extra"]);
    assert!(
        err.to_string()
            .contains("chic version does not accept additional arguments")
    );
}

#[test]
fn parses_header_command_without_options() {
    let cli = expect_cli_ok(["header", "library.cl"]);
    match cli.command {
        Command::Header {
            ref input,
            output,
            include_guard,
        } => {
            assert_eq!(input, Path::new("library.cl"));
            assert!(output.is_none());
            assert!(include_guard.is_none());
        }
        other => panic!("expected header command, found {other:?}"),
    }
}

#[test]
fn parses_header_command_with_options() {
    let cli = expect_cli_ok([
        "header",
        "api.cl",
        "-o",
        "api.h",
        "--include-guard",
        "API_GUARD",
    ]);
    match cli.command {
        Command::Header {
            ref input,
            output,
            include_guard,
        } => {
            assert_eq!(input, Path::new("api.cl"));
            assert_eq!(output.as_deref(), Some(Path::new("api.h")));
            assert_eq!(include_guard.as_deref(), Some("API_GUARD"));
        }
        other => panic!("expected header command, found {other:?}"),
    }
}

#[test]
fn header_command_rejects_unknown_flag() {
    let err = expect_cli_err(["header", "api.cl", "--unknown"]);
    assert!(err.to_string().contains("unsupported option"));
}

#[test]
fn parses_init_command() {
    let cli = expect_cli_ok(["init", "--template", "app", "MyApp"]);
    match cli.command {
        Command::Init {
            template,
            output,
            name,
        } => {
            assert_eq!(template, "app");
            assert_eq!(output, Some(PathBuf::from("MyApp")));
            assert!(name.is_none());
        }
        other => panic!("expected init command, found {other:?}"),
    }
}

#[test]
fn init_rejects_unknown_flag() {
    let err = expect_cli_err(["init", "--template", "app", "--unknown"]);
    assert!(err.to_string().contains("unsupported option"));
}

#[test]
fn parses_test_command() {
    let cli = expect_cli_ok(["test", "suite.cl", "--backend", "llvm"]);
    match cli.command {
        Command::Test { backend, .. } => assert_eq!(backend, Backend::Llvm),
        other => panic!("expected test command, found {other:?}"),
    }
}

#[test]
fn spec_rejects_extra_arguments() {
    let err = expect_cli_err(["spec", "extra"]);
    assert!(
        err.to_string()
            .contains("chic spec does not accept additional arguments")
    );
}

#[test]
fn parses_spec_command_and_alias() {
    let cli = expect_cli_ok(["spec"]);
    assert!(
        matches!(cli.command, Command::ShowSpec),
        "expected spec to map to ShowSpec command"
    );
    let alias = expect_cli_ok(["show-spec"]);
    assert!(
        matches!(alias.command, Command::ShowSpec),
        "show-spec alias should map to ShowSpec command"
    );
}

#[test]
fn unknown_command_reports_usage() {
    let err = expect_cli_err(["unknown"]);
    assert!(
        err.to_string().contains("unknown command 'unknown'"),
        "error message must surface the unknown command name"
    );
}

#[test]
fn format_rejects_stdin_with_inputs() {
    let err = expect_cli_err(["format", "--stdin", "main.cl"]);
    assert!(err.to_string().contains("cannot combine --stdin"));
}

#[test]
fn parses_format_command() {
    let cli = expect_cli_ok(["format", "main.cl", "extra.cl"]);
    match cli.command {
        Command::Format {
            ref inputs,
            check,
            diff,
            write,
            stdin,
            stdout,
            config,
        } => {
            assert_eq!(
                inputs,
                &vec![PathBuf::from("main.cl"), PathBuf::from("extra.cl")]
            );
            assert!(write);
            assert!(!check && !diff && !stdin && !stdout);
            assert!(config.is_none());
        }
        other => panic!("expected format command, found {other:?}"),
    }
}

#[test]
fn parses_format_check_and_diff_flags() {
    let cli = expect_cli_ok(["format", "--check", "--diff", "main.cl"]);
    match cli.command {
        Command::Format {
            check, diff, write, ..
        } => {
            assert!(check);
            assert!(diff);
            assert!(!write);
        }
        other => panic!("expected format command, found {other:?}"),
    }
}

#[test]
fn parses_wasm_backend() {
    let cli = expect_cli_ok(["build", "main.cl", "--backend", "wasm"]);
    match cli.command {
        Command::Build {
            backend, emit_wat, ..
        } => {
            assert_eq!(backend, Backend::Wasm);
            assert!(!emit_wat);
        }
        other => panic!("expected build command with wasm backend, found {other:?}"),
    }
}

#[test]
fn emit_wat_requires_wasm_backend() {
    let err = expect_cli_err(["build", "main.cl", "--emit-wat"]);
    let message = err.to_string();
    assert!(
        message.contains("--emit-wat requires --backend wasm"),
        "unexpected error message when backend missing: {message}"
    );
}

#[test]
fn parses_emit_wat_flag_for_wasm_build() {
    let cli = expect_cli_ok(["build", "main.cl", "--backend", "wasm", "--emit-wat"]);
    match cli.command {
        Command::Build {
            backend, emit_wat, ..
        } => {
            assert_eq!(backend, Backend::Wasm);
            assert!(emit_wat);
        }
        other => panic!("expected build command with emit-wat flag, found {other:?}"),
    }
}

#[test]
fn rejects_cranelift_backend() {
    let err = expect_cli_err(["build", "main.cl", "--backend", "cranelift"]);
    let message = err.to_string();
    assert!(
        message.contains("unsupported backend 'cranelift'"),
        "unexpected error message: {message}"
    );
    assert!(
        message.contains("expected llvm, wasm, or cc1"),
        "error should mention supported backends: {message}"
    );
}

#[test]
fn parses_cpu_isa_flag_for_build() {
    let cli = expect_cli_ok(["build", "main.cl", "--cpu-isa", "baseline,avx512"]);
    match cli.command {
        Command::Build { cpu_isa, .. } => {
            assert_eq!(
                cpu_isa.effective_tiers(TargetArch::X86_64),
                vec![CpuIsaTier::Baseline, CpuIsaTier::Avx512]
            );
        }
        other => panic!("expected build command, found {other:?}"),
    }
}

#[test]
fn parses_sve_bits_flag() {
    let cli = expect_cli_ok(["build", "main.cl", "--backend", "llvm", "--sve-bits", "256"]);
    match cli.command {
        Command::Build { cpu_isa, .. } => {
            assert_eq!(cpu_isa.sve_vector_bits(), Some(256));
        }
        other => panic!("expected build command, found {other:?}"),
    }
}

#[test]
fn parses_cpu_isa_flag_for_run() {
    let cli = expect_cli_ok(["run", "main.cl", "--cpu-isa", "auto"]);
    match cli.command {
        Command::Run { cpu_isa, .. } => {
            assert!(
                cpu_isa.tiers().contains(&CpuIsaTier::DotProd),
                "auto should include Apple tiers"
            );
            assert_eq!(
                cpu_isa.effective_tiers(TargetArch::X86_64),
                vec![
                    CpuIsaTier::Baseline,
                    CpuIsaTier::Avx2,
                    CpuIsaTier::Avx512,
                    CpuIsaTier::Amx
                ]
            );
            assert_eq!(
                cpu_isa.effective_tiers(TargetArch::Aarch64),
                vec![
                    CpuIsaTier::Baseline,
                    CpuIsaTier::DotProd,
                    CpuIsaTier::Fp16Fml,
                    CpuIsaTier::Bf16,
                    CpuIsaTier::I8mm,
                    CpuIsaTier::Sve,
                    CpuIsaTier::Sve2,
                    CpuIsaTier::Crypto,
                    CpuIsaTier::Pauth,
                    CpuIsaTier::Bti,
                    CpuIsaTier::Sme
                ]
            );
        }
        other => panic!("expected run command, found {other:?}"),
    }
}

#[test]
fn parses_cpu_isa_apple_profile() {
    let cli = expect_cli_ok(["build", "main.cl", "--cpu-isa", "apple-m2"]);
    match cli.command {
        Command::Build { cpu_isa, .. } => {
            assert_eq!(
                cpu_isa.effective_tiers(TargetArch::Aarch64),
                vec![
                    CpuIsaTier::Baseline,
                    CpuIsaTier::DotProd,
                    CpuIsaTier::Fp16Fml,
                    CpuIsaTier::Bf16,
                    CpuIsaTier::I8mm,
                    CpuIsaTier::Crypto,
                    CpuIsaTier::Pauth,
                    CpuIsaTier::Bti,
                ]
            );
        }
        other => panic!("expected build command, found {other:?}"),
    }
}

#[test]
fn rejects_unknown_cpu_isa_entry() {
    let err = expect_cli_err(["build", "main.cl", "--cpu-isa", "sparc"]);
    let message = err.to_string();
    assert!(
        message.contains("invalid ISA list"),
        "unexpected error: {message}"
    );
}

#[test]
fn rejects_invalid_sve_bits() {
    let err = expect_cli_err(["build", "main.cl", "--sve-bits", "63"]);
    let message = err.to_string();
    assert!(
        message.contains("--sve-bits must be a multiple of 128"),
        "unexpected error: {message}"
    );
}

#[test]
fn parse_seed_defaults_to_perf_json() {
    let cli = expect_cli_ok(["seed"]);
    match cli.command {
        Command::Seed {
            run_path,
            profile,
            json,
        } => {
            assert_eq!(run_path, PathBuf::from("perf.json"));
            assert!(profile.is_none());
            assert!(!json);
        }
        other => panic!("expected seed command, found {other:?}"),
    }
}

#[test]
fn parse_seed_accepts_profile_and_json() {
    let cli = expect_cli_ok([
        "seed",
        "--from-run",
        "runlog.json",
        "--profile",
        "debug",
        "--json",
    ]);
    match cli.command {
        Command::Seed {
            run_path,
            profile,
            json,
        } => {
            assert_eq!(run_path, PathBuf::from("runlog.json"));
            assert_eq!(profile.as_deref(), Some("debug"));
            assert!(json);
        }
        other => panic!("expected seed command with options, found {other:?}"),
    }
}
