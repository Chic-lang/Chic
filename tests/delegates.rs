use assert_cmd::cargo::cargo_bin_cmd;
use std::io::Write;
use tempfile::NamedTempFile;

fn write_source(contents: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temp file");
    file.write_all(contents.as_bytes())
        .expect("write temp file");
    file.flush().expect("flush");
    file
}

#[test]
fn delegate_from_lambda_executes() {
    let file = write_source(
        r#"
namespace Samples;

public delegate int IntUnary(int x);
public int Apply(IntUnary op, int value) => op(value);
public int Main()
{
    let square = (IntUnary)((int x) => x * x);
    return Apply(square, 5);
}
"#,
    );

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_RUN_ENTRY", "1")
        // Keep integration tests deterministic: CI defaults enable strict formatter enforcement,
        // which can cause tests that generate/inline Chic sources to fail before exercising the
        // behavior under test.
        .env("CHIC_CI", "0")
        .args(["run", file.path().to_str().unwrap(), "--backend", "wasm"])
        .assert()
        .code(25);
}

#[test]
fn delegate_method_group_executes() {
    let file = write_source(
        r#"
namespace Samples;

public delegate int BinaryOp(int a, int b);
public int Add(int a, int b) => a + b;
public int Main()
{
    let op = (BinaryOp)Add;
    return op(2, 3);
}
"#,
    );

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_RUN_ENTRY", "1")
        // Keep integration tests deterministic: CI defaults enable strict formatter enforcement,
        // which can cause tests that generate/inline Chic sources to fail before exercising the
        // behavior under test.
        .env("CHIC_CI", "0")
        .args(["run", file.path().to_str().unwrap(), "--backend", "wasm"])
        .assert()
        .code(5);
}

#[test]
fn delegate_variance_assignability() {
    let file = write_source(
        r#"
namespace Samples;

public delegate TResult Converter<in T, out TResult>(T value);
public class Base {}
public class Derived : Base {}

public int FromBase(Base b) => 1;
public int FromDerived(Derived d) => 2;

public int Main()
{
    let baseConv = (Converter<Base, int>)FromBase;
    let derivedConv = (Converter<Derived, int>)FromDerived;
    let cov = (Converter<Derived, int>)baseConv;
    let contra = (Converter<Base, int>)derivedConv;
    return cov(new Derived()) + contra(new Derived());
}
"#,
    );

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_RUN_ENTRY", "1")
        // Keep integration tests deterministic: CI defaults enable strict formatter enforcement,
        // which can cause tests that generate/inline Chic sources to fail before exercising the
        // behavior under test.
        .env("CHIC_CI", "0")
        .args(["run", file.path().to_str().unwrap(), "--backend", "wasm"])
        .assert()
        .code(3);
}
