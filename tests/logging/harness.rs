use assert_cmd::Command;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::fmt::Write;
use std::fs;
use std::path::Path;
use tempfile::{TempDir, tempdir};

const SAMPLE_PROGRAM: &str = r#"
namespace Logging;

testcase Works()
{
}

public class Program
{
    public int Main()
    {
        return 0;
    }
}
"#;

#[derive(Clone, Copy)]
pub(crate) enum Format {
    Text,
    Json,
}

impl Format {
    fn flag(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum CommandKind {
    Check,
    Build,
    Test,
    Run,
}

impl CommandKind {
    fn label(self) -> &'static str {
        match self {
            Self::Check => "chic check",
            Self::Build => "chic build",
            Self::Test => "chic test",
            Self::Run => "chic run",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum FilterKind {
    Stage { prefix: &'static str },
    Diagnostics,
}

impl FilterKind {
    pub(crate) fn stage(prefix: &'static str) -> Self {
        Self::Stage { prefix }
    }

    pub(crate) fn diagnostics() -> Self {
        Self::Diagnostics
    }

    fn matches(self, line: &str) -> bool {
        match self {
            Self::Stage { prefix } => {
                stage_value(line).map_or(false, |value| value.starts_with(prefix))
            }
            Self::Diagnostics => {
                let trimmed = line.trim_start();
                trimmed.starts_with("error:") || trimmed.starts_with("[Error]")
            }
        }
    }
}

pub(crate) fn collect_snapshot(
    commands: &[CommandKind],
    format: Format,
    filter: FilterKind,
) -> String {
    let fixture = Fixture::new();
    let mut buffer = String::new();
    for command in commands {
        let capture = fixture.run(*command, format);
        writeln!(buffer, "== {} ({}) ==", command.label(), format.flag()).unwrap();
        writeln!(buffer, "status: {:?}", capture.status).unwrap();
        let filtered = capture.filtered(filter);
        if filtered.trim().is_empty() {
            buffer.push_str("stderr:\n<none>\n\n");
        } else {
            buffer.push_str("stderr:\n");
            buffer.push_str(&filtered);
            buffer.push_str("\n\n");
        }
    }
    buffer.trim_end().to_string()
}

struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempdir().expect("create tempdir for logging snapshots");
        let source_path = dir.path().join("sample.cl");
        fs::write(&source_path, SAMPLE_PROGRAM).expect("write logging sample program");
        Self { dir }
    }

    fn run(&self, command: CommandKind, format: Format) -> CapturedLogs {
        let args = command_args(command, format);
        let output = run_command(&args, self.dir.path());
        let stderr_text = match format {
            Format::Text => {
                sanitize_text(&String::from_utf8_lossy(&output.stderr), self.dir.path())
            }
            Format::Json => {
                sanitize_json(&String::from_utf8_lossy(&output.stderr), self.dir.path())
            }
        };
        CapturedLogs {
            status: output.status.code(),
            stderr: stderr_text,
        }
    }
}

struct CapturedLogs {
    status: Option<i32>,
    stderr: String,
}

impl CapturedLogs {
    fn filtered(&self, filter: FilterKind) -> String {
        self.stderr
            .lines()
            .filter(|line| filter.matches(line))
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn command_args(command: CommandKind, format: Format) -> Vec<String> {
    let mut args = match command {
        CommandKind::Check => vec!["check".into(), "sample.cl".into()],
        CommandKind::Build => vec![
            "build".into(),
            "sample.cl".into(),
            "--backend".into(),
            "wasm".into(),
            "--output".into(),
            "out.wasm".into(),
        ],
        CommandKind::Test => vec![
            "test".into(),
            "sample.cl".into(),
            "--backend".into(),
            "wasm".into(),
        ],
        CommandKind::Run => vec![
            "run".into(),
            "sample.cl".into(),
            "--backend".into(),
            "wasm".into(),
        ],
    };
    args.push("--log-format".into());
    args.push(format.flag().into());
    args.push("--log-level".into());
    args.push("info".into());
    args
}

fn run_command(args: &[String], dir: &Path) -> std::process::Output {
    let mut cmd = Command::cargo_bin("chic").expect("chic binary");
    cmd.env("CHIC_SKIP_STDLIB", "1");
    cmd.current_dir(dir);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("execute chic command")
}

fn host_target_strings() -> (String, String) {
    let host = chic::target::Target::host().triple().to_string();
    let wasm = format!("{host}-wasm");
    (host, wasm)
}

fn sanitize_text(text: &str, dir: &Path) -> String {
    let path_repr = dir.to_string_lossy().replace('\\', "/");
    let mut normalized = text.replace('\\', "/");
    let ansi_re = Regex::new(r"\x1B\[[0-9;]*[A-Za-z]").expect("ansi regex");
    normalized = ansi_re.replace_all(&normalized, "").to_string();
    let timestamp_re =
        Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{6}Z").expect("timestamp regex");
    normalized = timestamp_re
        .replace_all(&normalized, "__TIMESTAMP__")
        .to_string();
    normalized = normalized.replace(&path_repr, "{TMP}");
    let elapsed_re = Regex::new(r"elapsed_ms\s*=\s*\d+").expect("elapsed regex");
    normalized = elapsed_re
        .replace_all(&normalized, "elapsed_ms=__ELAPSED__")
        .to_string();
    let run_tmp = Regex::new(r"chic-run-[A-Za-z0-9]+").expect("run tmp regex");
    normalized = run_tmp
        .replace_all(&normalized, "chic-run-{RANDOM}")
        .to_string();
    let (host_target, host_wasm_target) = host_target_strings();
    normalized = normalized.replace(&host_wasm_target, "aarch64-unknown-none");
    normalized = normalized.replace(&host_target, "aarch64-unknown-none");
    let artifact_re = Regex::new(r#"artifact="[^"]*/([^"/]+)""#).expect("artifact regex");
    normalized = artifact_re
        .replace_all(&normalized, r#"artifact="$1""#)
        .to_string();
    for (key, placeholder) in [
        ("module_count", "__MODULES__"),
        ("lowering_diagnostics", "__LOWERING__"),
        ("borrow_diagnostics", "__BORROW__"),
        ("type_diagnostics", "__TYPE__"),
        ("impls_checked", "__IMPLS__"),
        ("overlaps", "__OVERLAPS__"),
        ("traits_checked", "__TRAITS__"),
        ("cycles_detected", "__CYCLES__"),
    ] {
        let re = Regex::new(&format!(r"{key}=\d+")).expect("count regex");
        normalized = re
            .replace_all(&normalized, format!("{key}={placeholder}"))
            .to_string();
    }
    normalized
}

fn sanitize_json(stderr: &str, dir: &Path) -> String {
    let path_repr = dir.to_string_lossy().replace('\\', "/");
    stderr
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| match serde_json::from_str::<Value>(line) {
            Ok(mut value) => {
                scrub_json_value(&mut value, &path_repr);
                serde_json::to_string(&value).expect("json stringify")
            }
            Err(_) => sanitize_text(line, dir),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn scrub_json_value(value: &mut Value, path_repr: &str) {
    match value {
        Value::Object(map) => scrub_json_object(map, path_repr),
        Value::Array(items) => items
            .iter_mut()
            .for_each(|item| scrub_json_value(item, path_repr)),
        Value::String(text) => {
            *text = sanitize_json_string(text, path_repr);
        }
        _ => {}
    }
}

fn scrub_json_object(obj: &mut serde_json::Map<String, Value>, path_repr: &str) {
    if obj.contains_key("elapsed_ms") {
        obj.insert("elapsed_ms".into(), Value::from(0));
    }
    if obj.contains_key("timestamp") {
        obj.insert("timestamp".into(), Value::from("__TIMESTAMP__"));
    }
    for key in [
        "module_count",
        "lowering_diagnostics",
        "borrow_diagnostics",
        "type_diagnostics",
        "impls_checked",
        "overlaps",
        "traits_checked",
        "cycles_detected",
        "input_count",
        "inputs",
    ] {
        if let Some(value) = obj.get_mut(key) {
            if value.is_number() {
                obj.insert(key.into(), Value::from("__COUNT__"));
            }
        }
    }
    if let Some(Value::String(artifact)) = obj.get_mut("artifact") {
        if let Some((_, basename)) = artifact.rsplit_once('/') {
            *artifact = basename.to_string();
        }
    }
    for value in obj.values_mut() {
        scrub_json_value(value, path_repr);
    }
}

fn sanitize_json_string(input: &str, path_repr: &str) -> String {
    static ANSI_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"\x1B\[[0-9;]*[A-Za-z]").expect("ansi regex"));
    static RUN_TMP: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"chic-run-[A-Za-z0-9]+").expect("run tmp regex"));

    let mut text = input.replace('\\', "/");
    text = ANSI_RE.replace_all(&text, "").to_string();
    text = text.replace(path_repr, "{TMP}");
    text = RUN_TMP.replace_all(&text, "chic-run-{RANDOM}").to_string();
    let (host_target, host_wasm_target) = host_target_strings();
    text = text.replace(&host_wasm_target, "aarch64-unknown-none");
    text = text.replace(&host_target, "aarch64-unknown-none");
    text
}

fn stage_value(line: &str) -> Option<&str> {
    if let Some(idx) = line.find("stage=\"") {
        let rest = &line[idx + 7..];
        if let Some(end) = rest.find('"') {
            return Some(&rest[..end]);
        }
    }
    if let Some(idx) = line.find("\"stage\":\"") {
        let rest = &line[idx + 9..];
        if let Some(end) = rest.find('"') {
            return Some(&rest[..end]);
        }
    }
    None
}

macro_rules! log_snapshot_test {
    ($name:ident, $format:expr, $filter:expr, $commands:expr, $expect:expr) => {
        #[test]
        fn $name() {
            let actual = crate::harness::collect_snapshot($commands, $format, $filter);
            $expect.assert_eq(&actual);
        }
    };
}
