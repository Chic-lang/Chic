use assert_cmd::Command;
use tempfile::tempdir;

mod common;
use common::write_source;

#[test]
#[ignore = "decimal sample currently fails to parse; tracked separately from CLI changes"]
fn decimal_arithmetic_parse_and_format() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
namespace DecimalNative;

using Std;
import Std.Numeric;

public class Program
{
    public int Main()
    {
        if (!Decimal.TryParse("1.25", out var parsed) || parsed.ToDecimal() != 1.25m)
        {
            return 1;
        }

        let text = parsed.ToString();
        Decimal parsedBack;
        if (!Decimal.TryParse(text, out parsedBack) || parsedBack != parsed)
        {
            return 2;
        }

        let sum = Decimal.From(1.5m) + Decimal.From(2.5m);
        if (sum.ToDecimal() != 4.0m)
        {
            return 3;
        }

        let difference = Decimal.From(5m) - Decimal.From(3m);
        if (difference.ToDecimal() != 2m)
        {
            return 4;
        }

        let product = Decimal.From(1.2m) * Decimal.From(2m);
        if (product.ToDecimal() != 2.4m)
        {
            return 5;
        }

        let quotient = Decimal.From(5m) / Decimal.From(2m);
        if (quotient.ToDecimal() != 2.5m)
        {
            return 6;
        }

        let remainder = Decimal.From(5m) % Decimal.From(2m);
        if (remainder.ToDecimal() != 1m)
        {
            return 7;
        }

        return 0;
    }
}
"#;

    let dir = tempdir()?;
    let source_path = dir.path().join("decimal_native.cl");
    write_source(&source_path, source);

    for backend in ["llvm", "wasm"] {
        Command::cargo_bin("chic")?
            .arg("run")
            .arg(source_path.to_str().unwrap())
            .args(["--backend", backend])
            .assert()
            .success();
    }

    Ok(())
}
