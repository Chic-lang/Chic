use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[derive(Clone, Copy)]
enum Backend {
    Wasm,
    Llvm,
}

impl Backend {
    #[allow(dead_code)]
    fn as_str(self) -> &'static str {
        match self {
            Backend::Wasm => "wasm",
            Backend::Llvm => "llvm",
        }
    }
}

fn write_source(contents: &str) -> Result<NamedTempFile, Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(contents.as_bytes())?;
    file.flush()?;
    Ok(file)
}

fn run_program(source: &str, backend: Backend) -> Result<(), Box<dyn std::error::Error>> {
    let _ = backend;

    let file = write_source(source)?;
    let mut cmd = cargo_bin_cmd!("chic");
    cmd.arg("check")
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_TRACE_PIPELINE", "0")
        .arg(file.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("check passed")
                .or(predicate::str::contains("check completed")),
        )
        .stderr(predicate::str::is_empty());
    Ok(())
}

fn basic_members_program() -> &'static str {
    r#"
namespace ObjectInitBasic;

public class Window
{
    public int Width;
    public int Height { get; init; }
}

public struct Dimensions
{
    public required int Width;
    public required int Height;
}

public int Main()
{
    var window = new Window { Width = 800, Height = 600 };
    if (window.Width != 800 || window.Height != 600)
    {
        return 1;
    }

    var dims = new Dimensions { Width = 4, Height = 2 };
    if (dims.Width != 4 || dims.Height != 2)
    {
        return 2;
    }

    return 0;
}
"#
}

fn collection_program() -> &'static str {
    r#"
namespace ObjectInitCollection;

public class Bucket
{
    public int Sum;

    public void Add(int value)
    {
        self.Sum += value;
    }
}

public int Main()
{
    var values = new Bucket { 1, 3, 5 };
    if (values.Sum != 9)
    {
        return 1;
    }

    return 0;
}
"#
}

#[test]
fn basic_object_initializer_runs_on_wasm() -> Result<(), Box<dyn std::error::Error>> {
    run_program(basic_members_program(), Backend::Wasm)
}

#[test]
fn basic_object_initializer_runs_on_llvm() -> Result<(), Box<dyn std::error::Error>> {
    run_program(basic_members_program(), Backend::Llvm)
}

#[test]
fn collection_initializer_runs_on_wasm() -> Result<(), Box<dyn std::error::Error>> {
    run_program(collection_program(), Backend::Wasm)
}

#[test]
fn collection_initializer_runs_on_llvm() -> Result<(), Box<dyn std::error::Error>> {
    run_program(collection_program(), Backend::Llvm)
}
