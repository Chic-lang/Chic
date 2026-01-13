use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

mod common;
use common::write_source;

#[test]
#[ignore = "stdlib build currently fails (region profile identifiers); tracked separately"]
fn generic_math_resolves_via_interfaces() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
namespace GenericMath;

using Std;
import Std.Numeric;

public class Program
{
    private static T Add<T>(T a, T b) where T : INumber<T>
    {
        return a + b;
    }

    private static T Increment<T>(T value) where T : INumber<T>
    {
        value++;
        return value;
    }

    private static T BitAnd<T>(T left, T right) where T : IBinaryInteger<T>
    {
        return left & right;
    }

    private static T ShiftLeft<T>(T value, int offset) where T : IBinaryInteger<T>
    {
        return value << offset;
    }

    private static T Negate<T>(T value) where T : ISignedNumber<T>
    {
        return -value;
    }

    public int Main()
    {
        if (Add(Int32.From(2), Int32.From(3)).ToInt32() != 5)
        {
            return 1;
        }

        if (Increment(Int16.From(4)).ToInt16() != 5)
        {
            return 2;
        }

        if (Negate(Int64.From(-9)).ToInt64() != 9)
        {
            return 3;
        }

        if (BitAnd(UInt32.From(0xF0u), UInt32.From(0x0Fu)).ToUInt32() != 0u)
        {
            return 4;
        }

        if (ShiftLeft(UInt64.From(1ul), 4).ToUInt64() != 16ul)
        {
            return 5;
        }

        let fp = Add(Float32.From(1.5f), Float32.From(2.5f));
        if (fp.ToFloat32() != 4.0f)
        {
            return 6;
        }

        let fpNegated = Negate(Float64.From(-0.5d));
        if (fpNegated.ToFloat64() != 0.5d)
        {
            return 7;
        }

        let wide = Add(Int128.From(10L), Int128.From(5L));
        if (wide.ToInt128() != 15L)
        {
            return 8;
        }

        let wideUnsigned = ShiftLeft(UInt128.From(1ul), 1);
        if (wideUnsigned.ToUInt128() != 2ul)
        {
            return 9;
        }

        let dec = Add(Decimal.From(1.5m), Decimal.From(2.5m));
        if (dec.ToDecimal() != 4.0m)
        {
            return 10;
        }

        let decNegated = Negate(Decimal.From(-1.25m));
        if (decNegated.ToDecimal() != 1.25m)
        {
            return 11;
        }

        return 0;
    }
}
"#;

    let dir = tempdir()?;
    let source_path = dir.path().join("generic_math.cl");
    write_source(&source_path, source);

    for backend in ["llvm", "wasm"] {
        cargo_bin_cmd!("chic")
            .arg("run")
            .arg(source_path.to_str().unwrap())
            .args(["--backend", backend])
            .assert()
            .success();
    }

    Ok(())
}
