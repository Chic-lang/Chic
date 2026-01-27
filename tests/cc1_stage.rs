use assert_cmd::cargo::cargo_bin_cmd;

mod common;
use common::{clang_available, write_source};

#[test]
fn cc1_command_emits_assembly() {
    if !clang_available() {
        eprintln!("skipping cc1 command test: clang not available");
        return;
    }

    let dir = tempfile::tempdir().expect("temp dir");
    let input = dir.path().join("sample.i");
    write_source(
        &input,
        "int add(int a, int b) { return a + b; }\nint main(void) { return add(2, 3); }\n",
    );

    cargo_bin_cmd!("chic")
        .arg("cc1")
        .arg(&input)
        .assert()
        .success()
        .stdout(predicates::str::contains("cc1 emitted assembly"));

    let output = input.with_extension("s");
    assert!(
        output.exists(),
        "expected assembly file at {}",
        output.display()
    );
    let contents = std::fs::read_to_string(&output).expect("read assembly");
    assert!(
        contents.contains("add"),
        "expected symbol name in assembly: {}",
        contents
    );
}

#[test]
fn cc1_backend_generates_chic_assembly() {
    if !clang_available() {
        eprintln!("skipping cc1 backend test: clang not available");
        return;
    }

    let dir = tempfile::tempdir().expect("temp dir");
    let src_path = dir.path().join("program.ch");
    write_source(
        &src_path,
        r"
namespace Cc1;

/// <summary>Simple entry point used to validate the cc1 backend.</summary>
public int Main()
{
    return 20 + 22;
}
",
    );

    let asm_path = dir.path().join("program.s");
    cargo_bin_cmd!("chic")
        .arg("build")
        .arg(&src_path)
        .args([
            "--backend",
            "cc1",
            "-o",
            asm_path.to_str().expect("utf8 path"),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("frontend succeeded"));

    assert!(
        asm_path.exists(),
        "assembly output missing at {}",
        asm_path.display()
    );

    let object_path = dir.path().join("program.o");
    let status = std::process::Command::new("clang")
        .arg("-c")
        .arg(&asm_path)
        .arg("-o")
        .arg(&object_path)
        .status()
        .expect("assemble cc1 output");
    assert!(
        status.success(),
        "clang failed to assemble cc1 output: {status:?}"
    );
    assert!(
        object_path.exists(),
        "expected assembled object at {}",
        object_path.display()
    );
}
