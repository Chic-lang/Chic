use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

mod common;
use common::write_source;

#[test]
fn generic_numeric_constraints_work() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
namespace GenericMath;

public static class Helpers
{
    public static T Identity<T>(T value)
    {
        return value;
    }
}

public class Program
{
    public int Main()
    {
        let number = Helpers.Identity(3);
        return number == 3 ? 0 : 1;
    }
}
"#;

    let dir = tempdir()?;
    let source_path = dir.path().join("generic_numeric.cl");
    write_source(&source_path, source);

    for backend in ["llvm"] {
        cargo_bin_cmd!("chic")
            .arg("run")
            .arg(source_path.to_str().unwrap())
            .args(["--backend", backend])
            .env("CHIC_SKIP_STDLIB", "1")
            .assert()
            .success();
    }

    Ok(())
}
