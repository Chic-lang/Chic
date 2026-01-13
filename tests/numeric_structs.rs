use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::env;
use tempfile::tempdir;

mod common;
use common::write_source;

#[test]
fn numeric_structs_behave_across_backends() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("CHIC_ENABLE_NUMERIC_STRUCTS").is_err() {
        eprintln!(
            "skipping numeric_structs_behave_across_backends (set CHIC_ENABLE_NUMERIC_STRUCTS=1 to enable)"
        );
        return Ok(());
    }

    let source = r#"
namespace NumericTests;

using Std;
import Std.Numeric;

public class Program
{
    public int Main()
    {
        var parsed = Int32.Parse("123");
        if (parsed.ToInt32() != 123)
        {
            return 1;
        }

        if (!Int32.TryParse(" -2147483648 ", out parsed) || parsed.ToInt32() != Int32.MinValue)
        {
            return 2;
        }

        if (!Int32.TryParse("+99", out parsed) || parsed.ToInt32() != 99)
        {
            return 3;
        }

        if (Int32.TryParse("not a number", out parsed))
        {
            return 4;
        }

        if (Int32.TryParse("2147483648", out parsed))
        {
            return 5;
        }

        var intClone = Int32.From(456);
        var intCloneText = intClone.ToString();
        if (!Int32.TryParse(intCloneText, out parsed) || parsed.ToInt32() != intClone.ToInt32())
        {
            return 6;
        }

        if (Int32.Compare(Int32.From(456), Int32.From(123)) <= 0)
        {
            return 7;
        }
        if (Int32.Compare(Int32.From(123), Int32.From(123)) != 0)
        {
            return 8;
        }
        if (!Int32.Equals(Int32.From(-5), Int32.From(-5)))
        {
            return 9;
        }

        var parsed64 = Int64.From(0L);
        if (!Int64.TryParse("9223372036854775807", out parsed64)
            || parsed64.ToInt64() != Int64.MaxValue)
        {
            return 10;
        }

        if (Int64.TryParse("-9223372036854775809", out parsed64))
        {
            return 11;
        }

        var int64Text = Int64.From(-9000000000000000000L).ToString();
        if (!Int64.TryParse(int64Text, out parsed64) || parsed64.ToInt64() != -9000000000000000000L)
        {
            return 12;
        }

        var parsedU32 = UInt32.From(0u);
        if (!UInt32.TryParse("4294967295", out parsedU32)
            || parsedU32.ToUInt32() != UInt32.MaxValue)
        {
            return 13;
        }

        if (UInt32.TryParse("-1", out parsedU32))
        {
            return 14;
        }

        var u32Text = UInt32.From(0u).ToString();
        if (!UInt32.TryParse(u32Text, out parsedU32) || parsedU32.ToUInt32() != 0u)
        {
            return 15;
        }

        var parsedU64 = UInt64.From(0ul);
        if (!UInt64.TryParse("18446744073709551615", out parsedU64)
            || parsedU64.ToUInt64() != UInt64.MaxValue)
        {
            return 16;
        }

        if (UInt64.TryParse("-42", out parsedU64))
        {
            return 17;
        }

        if (!UInt64.TryParse(" +42 ", out parsedU64) || parsedU64.ToUInt64() != 42ul)
        {
            return 18;
        }

        var u64Text = UInt64.From(500015ul).ToString();
        if (!UInt64.TryParse(u64Text, out parsedU64) || parsedU64.ToUInt64() != 500015ul)
        {
            return 19;
        }

        if (UInt64.Compare(UInt64.From(5ul), UInt64.From(6ul)) >= 0)
        {
            return 20;
        }

        var sByteValue = SByte.Parse("-128");
        if (sByteValue.ToSByte() != SByte.MinValue)
        {
            return 21;
        }

        if (SByte.TryParse("128", out sByteValue))
        {
            return 22;
        }

        if (!SByte.TryParse(" +42 ", out sByteValue) || sByteValue.ToSByte() != 42)
        {
            return 23;
        }

        if (SByte.Compare(SByte.From(-5), SByte.From(4)) >= 0)
        {
            return 24;
        }

        var parsedByte = Byte.From(0u8);
        if (!Byte.TryParse("255", out parsedByte) || parsedByte.ToByte() != Byte.MaxValue)
        {
            return 25;
        }

        if (Byte.TryParse("256", out parsedByte))
        {
            return 26;
        }

        var byteText = Byte.From(200u8).ToString();
        if (!Byte.TryParse(byteText, out parsedByte) || parsedByte.ToByte() != 200u8)
        {
            return 27;
        }

        var productByte = Byte.Zero;
        if (!Byte.TryMultiply(Byte.From(12u8), Byte.From(11u8), out productByte)
            || productByte.ToByte() != 132u8)
        {
            return 82;
        }

        if (Byte.LeadingZeroCount(Byte.From(0b0001_0000u8)) != 3)
        {
            return 83;
        }

        if (Byte.TrailingZeroCount(Byte.From(0b0010_0000u8)) != 5)
        {
            return 84;
        }

        if (Byte.PopCount(Byte.From(0xF0u8)) != 4)
        {
            return 85;
        }

        if (Byte.RotateLeft(Byte.From(0x12u8), 4).ToByte() != 0x21u8)
        {
            return 86;
        }

        if (!Byte.IsPowerOfTwo(Byte.From(8u8)))
        {
            return 87;
        }

        if (Byte.IsPowerOfTwo(Byte.From(0u8)))
        {
            return 88;
        }

        var parsed16 = Int16.From(0);
        if (!Int16.TryParse("32_767", out parsed16) || parsed16.ToInt16() != Int16.MaxValue)
        {
            return 28;
        }

        if (Int16.TryParse("32768", out parsed16))
        {
            return 29;
        }

        var parsedU16 = UInt16.From(0u16);
        if (!UInt16.TryParse("65_535", out parsedU16) || parsedU16.ToUInt16() != UInt16.MaxValue)
        {
            return 30;
        }

        if (UInt16.TryParse("65536", out parsedU16))
        {
            return 31;
        }

        var shortText = Int16.From(-12345).ToString();
        if (!Int16.TryParse(shortText, out parsed16) || parsed16.ToInt16() != -12345)
        {
            return 32;
        }

        if (UInt16.Compare(UInt16.From(15u16), UInt16.From(14u16)) <= 0)
        {
            return 33;
        }

        var negated16 = Int16.Zero;
        if (!Int16.TryNegate(Int16.From(-12345), out negated16) || negated16.ToInt16() != 12345)
        {
            return 89;
        }

        if (Int16.TryNegate(Int16.From(Int16.MinValue), out negated16))
        {
            return 90;
        }

        if (Int16.RotateLeft(Int16.From(0x4001), 1).ToInt16() != 0x8002)
        {
            return 91;
        }

        if (Int16.LeadingZeroCount(Int16.From(0x0400)) != 5)
        {
            return 92;
        }

        if (!Int16.IsPowerOfTwo(Int16.From(1024)))
        {
            return 93;
        }

        var productU16 = UInt16.Zero;
        if (!UInt16.TryMultiply(UInt16.From(256u16), UInt16.From(255u16), out productU16)
            || productU16.ToUInt16() != 65280u16)
        {
            return 94;
        }

        if (UInt16.LeadingZeroCount(UInt16.From(0b0000_0001_0000_0000u16)) != 7)
        {
            return 95;
        }

        if (UInt16.TrailingZeroCount(UInt16.From(0b0100_0000_0000_0000u16)) != 14)
        {
            return 96;
        }

        if (UInt16.PopCount(UInt16.From(0x0FF0u16)) != 8)
        {
            return 97;
        }

        if (UInt16.RotateRight(UInt16.From(0x8001u16), 1).ToUInt16() != 0xC000u16)
        {
            return 98;
        }

        if (UInt16.ReverseEndianness(UInt16.From(0x1234u16)).ToUInt16() != 0x3412u16)
        {
            return 99;
        }

        if (!UInt16.IsPowerOfTwo(UInt16.From(2048u16)))
        {
            return 100;
        }

        if (UInt16.IsPowerOfTwo(UInt16.From(0u16)))
        {
            return 101;
        }

        var pointerBits = NumericInfo.PointerBitWidth;
        var parsedPtr = IntPtr.Zero;
        if (!IntPtr.TryParse("42", out parsedPtr) || parsedPtr.ToIntPtr() != (nint)42)
        {
            return 34;
        }

        var ptrText = IntPtr.From((nint)(-17)).ToString();
        if (!IntPtr.TryParse(ptrText, out parsedPtr) || parsedPtr.ToIntPtr() != (nint)(-17))
        {
            return 35;
        }

        if (pointerBits == 32u)
        {
            if (IntPtr.TryParse("4294967296", out parsedPtr))
            {
                return 36;
            }
        }
        else
        {
            if (!IntPtr.TryParse("9223372036854775807", out parsedPtr)
                || parsedPtr.ToIntPtr() != (nint)9223372036854775807L)
            {
                return 37;
            }
        }

        var parsedUPtr = UIntPtr.Zero;
        if (!UIntPtr.TryParse("84", out parsedUPtr) || parsedUPtr.ToUIntPtr() != (nuint)84)
        {
            return 38;
        }

        var uPtrText = UIntPtr.From((nuint)512).ToString();
        if (!UIntPtr.TryParse(uPtrText, out parsedUPtr) || parsedUPtr.ToUIntPtr() != (nuint)512)
        {
            return 39;
        }

        if (pointerBits == 32u)
        {
            if (UIntPtr.TryParse("4294967296", out parsedUPtr))
            {
                return 102;
            }
        }
        else
        {
            if (!UIntPtr.TryParse("18446744073709551615", out parsedUPtr)
                || parsedUPtr.ToUIntPtr() != (nuint)18446744073709551615ul)
            {
                return 103;
            }
        }

        var summedUPtr = UIntPtr.Zero;
        if (!UIntPtr.TryAdd(UIntPtr.From((nuint)12), UIntPtr.From((nuint)30), out summedUPtr)
            || summedUPtr.ToUIntPtr() != (nuint)42)
        {
            return 104;
        }

        if (UIntPtr.RotateRight(UIntPtr.From((nuint)1), 1).ToUIntPtr()
            != (pointerBits == 32u ? (nuint)0x80000000u : (nuint)0x8000000000000000ul))
        {
            return 105;
        }

        var sumChecked = Int32.Zero;
        if (!Int32.TryAdd(Int32.From(2000000000), Int32.From(147483647), out sumChecked)
            || sumChecked.ToInt32() != 2147483647)
        {
            return 43;
        }

        if (Int32.TryAdd(Int32.From(Int32.MaxValue), Int32.From(1), out sumChecked))
        {
            return 44;
        }

        if (!Int32.TryMultiply(Int32.From(-12), Int32.From(7), out sumChecked)
            || sumChecked.ToInt32() != -84)
        {
            return 45;
        }

        var negated32 = Int32.Zero;
        if (!Int32.TryNegate(Int32.From(-123), out negated32) || negated32.ToInt32() != 123)
        {
            return 46;
        }

        if (Int32.TryNegate(Int32.From(Int32.MinValue), out negated32))
        {
            return 47;
        }

        if (Int32.LeadingZeroCount(Int32.From(0b0001_0000)) != 27)
        {
            return 48;
        }

        if (Int32.TrailingZeroCount(Int32.From(0b0100_0000)) != 6)
        {
            return 49;
        }

        if (Int32.PopCount(Int32.From(-1)) != 32)
        {
            return 50;
        }

        if (Int32.RotateLeft(Int32.From(1), 5).ToInt32() != 32)
        {
            return 51;
        }

        if (Int32.RotateRight(Int32.From(16), 1).ToInt32() != 8)
        {
            return 52;
        }

        if (Int32.ReverseEndianness(Int32.From(0x01020304)).ToInt32() != 0x04030201)
        {
            return 53;
        }

        if (!Int32.IsPowerOfTwo(Int32.From(1024)))
        {
            return 54;
        }

        if (Int32.IsPowerOfTwo(Int32.From(-4)))
        {
            return 55;
        }

        if (Int32.From(255).ToString("X") != "FF")
        {
            return 56;
        }

        if (Int32.From(-1).ToString("x") != "ffffffff")
        {
            return 57;
        }

        var sumCheckedU = UInt32.Zero;
        if (!UInt32.TryAdd(UInt32.From(4000000000u), UInt32.From(255u), out sumCheckedU)
            || sumCheckedU.ToUInt32() != 4000000255u)
        {
            return 58;
        }

        if (UInt32.TryAdd(UInt32.From(UInt32.MaxValue), UInt32.From(1u), out sumCheckedU))
        {
            return 59;
        }

        if (UInt32.RotateRight(UInt32.From(1u), 1).ToUInt32() != 0x80000000u)
        {
            return 60;
        }

        if (UInt32.ReverseEndianness(UInt32.From(0xAABBCCDDu)).ToUInt32() != 0xDDCCBBAAu)
        {
            return 61;
        }

        if (!UInt32.IsPowerOfTwo(UInt32.From(512u)))
        {
            return 62;
        }

        if (UInt32.IsPowerOfTwo(UInt32.From(0u)))
        {
            return 63;
        }

        if (UInt32.From(255u).ToString("b") != "11111111")
        {
            return 64;
        }

        if (Int64.RotateLeft(Int64.From(1L), 63).ToInt64() != Int64.MinValue)
        {
            return 65;
        }

        if (Int64.PopCount(Int64.From(-1L)) != 64)
        {
            return 66;
        }

        if (UInt64.RotateLeft(UInt64.From(1ul), 63).ToUInt64() != 0x8000000000000000ul)
        {
            return 67;
        }

        if (!UInt64.IsPowerOfTwo(UInt64.From(1ul << 40)))
        {
            return 68;
        }

        if (UInt64.From(0xFFul).ToString("X") != "FF")
        {
            return 69;
        }

        var summedPtr = IntPtr.Zero;
        if (!IntPtr.TryAdd(IntPtr.From((nint)32), IntPtr.From((nint)10), out summedPtr)
            || summedPtr.ToIntPtr() != (nint)42)
        {
            return 70;
        }

        if (IntPtr.TryNegate(IntPtr.From(IntPtr.MinValue), out summedPtr))
        {
            return 71;
        }

        if (IntPtr.RotateLeft(IntPtr.From((nint)1), 1).ToIntPtr() != (nint)2)
        {
            return 72;
        }

        var reversedPtr = IntPtr.ReverseEndianness(IntPtr.From((nint)0x01020304));
        if (pointerBits == 32u)
        {
            if (reversedPtr.ToIntPtr() != (nint)0x04030201)
            {
                return 73;
            }
        }
        else
        {
            if (reversedPtr.ToIntPtr() != (nint)0x0403020100000000L)
            {
                return 73;
            }
        }

        if (IntPtr.LeadingZeroCount(IntPtr.From((nint)1)) != (int)(pointerBits - 1))
        {
            return 74;
        }

        if (!UIntPtr.TryAdd(UIntPtr.From((nuint)21), UIntPtr.From((nuint)21), out parsedUPtr)
            || parsedUPtr.ToUIntPtr() != (nuint)42)
        {
            return 75;
        }

        if (UIntPtr.TryAdd(UIntPtr.From(UIntPtr.MaxValue), UIntPtr.From((nuint)1), out parsedUPtr))
        {
            return 76;
        }

        if (UIntPtr.RotateRight(UIntPtr.From((nuint)2), 1).ToUIntPtr() != (nuint)1)
        {
            return 77;
        }

        if (UIntPtr.RotateLeft(UIntPtr.From((nuint)1), 1).ToUIntPtr() != (nuint)2)
        {
            return 102;
        }

        if (!UIntPtr.IsPowerOfTwo(UIntPtr.From((nuint)128)))
        {
            return 78;
        }

        if (UIntPtr.LeadingZeroCount(UIntPtr.From((nuint)1)) != (int)(pointerBits - 1))
        {
            return 103;
        }

        var reversedUPtr = UIntPtr.ReverseEndianness(UIntPtr.From((nuint)0x01020304));
        if (pointerBits == 32u)
        {
            if (reversedUPtr.ToUIntPtr() != (nuint)0x04030201)
            {
                return 104;
            }
        }
        else
        {
            if (reversedUPtr.ToUIntPtr() != (nuint)0x0403020100000000ul)
            {
                return 104;
            }
        }

        var ptrHex = IntPtr.From((nint)(-1)).ToString("X");
        if (pointerBits == 32u)
        {
            if (ptrHex != "FFFFFFFF")
            {
                return 79;
            }
        }
        else
        {
            if (ptrHex != "FFFFFFFFFFFFFFFF")
            {
                return 79;
            }
        }

        var uPtrBinary = UIntPtr.From(UIntPtr.MaxValue).ToString("b");
        if (pointerBits == 32u)
        {
            if (uPtrBinary.Length != 32)
            {
                return 80;
            }
        }
        else
        {
            if (uPtrBinary.Length != 64)
            {
                return 80;
            }
        }

        if (pointerBits == 32u)
        {
            if (UIntPtr.TryParse("4294967296", out parsedUPtr))
            {
                return 40;
            }
        }
        else
        {
            if (!UIntPtr.TryParse("18446744073709551615", out parsedUPtr)
                || parsedUPtr.ToUIntPtr() != (nuint)18446744073709551615ul)
            {
                return 41;
            }
        }

        if (UIntPtr.Compare(UIntPtr.From((nuint)1), UIntPtr.From((nuint)0)) <= 0)
        {
            return 81;
        }

        unsafe
        {
            let pointerValue = 1234;
            var pointer = &pointerValue;
            var handle = UIntPtr.FromPointer(pointer);
            var roundtrip = handle.AsPointer<int>();
            if (roundtrip != pointer)
            {
                return 105;
            }

            let constPointer = UIntPtr.FromPointer(pointer).AsConstPointer<int>();
            var constHandle = UIntPtr.FromConstPointer(constPointer);
            if (constHandle.AsConstPointer<int>() != constPointer)
            {
                return 106;
            }

            var address = UIntPtr.AddressOf(pointer);
            if (address == (nuint)0)
            {
                return 107;
            }

            let addressRoundtrip = UIntPtr.PointerFromAddress<int>(address);
            if (addressRoundtrip != pointer)
            {
                return 108;
            }

            var constAddress = UIntPtr.AddressOfConst(constPointer);
            let constRoundtrip = UIntPtr.PointerFromConstAddress<int>(constAddress);
            if (constRoundtrip != constPointer)
            {
                return 111;
            }
        }

        return 0;
    }
}
"#;

    let dir = tempdir()?;
    let source_path = dir.path().join("numeric_structs.cl");
    write_source(&source_path, source);

    for backend in ["llvm", "wasm"] {
        cargo_bin_cmd!("chic")
            .arg("run")
            .arg(source_path.to_str().unwrap())
            .args(["--backend", backend])
            .assert()
            .success()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::is_empty().or(predicate::str::contains(
                "warning: overriding the module target triple",
            )));
    }

    Ok(())
}
