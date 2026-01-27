namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;

testcase Given_char_classification_and_mapping_When_executed_Then_char_classification_and_mapping()
{
    let scalar = chic_rt_char_is_scalar((ushort) 'A');
    let digit = chic_rt_char_is_digit((ushort) '7');
    let letter = chic_rt_char_is_letter((ushort) 'z');
    let whitespace = chic_rt_char_is_whitespace(0x20);
    let upper = chic_rt_char_to_upper((ushort) 'b');
    let upperStatus = chic_rt_char_status(upper);
    let upperValue = (char) chic_rt_char_value(upper);
    let lower = chic_rt_char_to_lower((ushort) 'Q');
    let lowerStatus = chic_rt_char_status(lower);
    let lowerValue = (char) chic_rt_char_value(lower);
    let from = chic_rt_char_from_codepoint((uint) 'X');
    let fromStatus = chic_rt_char_status(from);
    let fromValue = (char) chic_rt_char_value(from);
    let ok = scalar == 1
        && digit == 1
        && letter == 1
        && whitespace == 1
        && upperStatus == 0
        && upperValue == 'B'
        && lowerStatus == 0
        && lowerValue == 'q'
        && fromStatus == 0
        && fromValue == 'X';
    Assert.That(ok).IsTrue();
}

testcase Given_char_invalid_scalar_inputs_When_executed_Then_char_invalid_scalar_inputs()
{
    let scalar = chic_rt_char_is_scalar((ushort) 0xD800);
    let digit = chic_rt_char_is_digit((ushort) 0xD800);
    let letter = chic_rt_char_is_letter((ushort) 0xDFFF);
    let whitespace = chic_rt_char_is_whitespace((ushort) 0xD800);
    let invalid = chic_rt_char_from_codepoint(0x110000u);
    let ok = scalar == 0
        && digit == -1
        && letter == -1
        && whitespace == -1
        && chic_rt_char_status(invalid) == 1;
    Assert.That(ok).IsTrue();
}

testcase Given_float_env_rounding_and_flags_When_executed_Then_float_env_rounding_and_flags()
{
    chic_rt_float_flags_clear();
    let flags = chic_rt_float_flags_read();
    let setOk = chic_rt_set_rounding_mode(0);
    let setBad = chic_rt_set_rounding_mode(5);
    let mode = chic_rt_rounding_mode();
    let ok = flags == 0u && setOk == 0 && setBad == -1 && mode >= 0;
    Assert.That(ok).IsTrue();
}

testcase Given_float_env_records_flags_When_executed_Then_float_env_records_flags()
{
    FloatEnv.Clear();
    let flags = new FloatFlags {
        Invalid = true, DivideByZero = false, Overflow = true, Underflow = false, Inexact = true,
    }
    ;
    FloatEnv.Record(flags);
    let mask = chic_rt_float_flags_read();
    chic_rt_float_flags_clear();
    let cleared = chic_rt_float_flags_read();
    let ok = (mask & 0x1u) != 0u && (mask & 0x4u) != 0u && (mask & 0x10u) != 0u && cleared == 0u;
    Assert.That(ok).IsTrue();
}

testcase Given_zero_init_clears_memory_When_executed_Then_zero_init_clears_memory()
{
    unsafe {
        var block = MemoryRuntime.chic_rt_alloc(4usize, 1usize);
        var idx = 0usize;
        while (idx <4usize)
        {
            let ptr = NativePtr.OffsetMut(block.Pointer, (isize) idx);
            * ptr = 0xFFu8;
            idx = idx + 1;
        }
        ZeroInit.chic_rt_zero_init(block.Pointer, 4usize);
        var ok = true;
        idx = 0usize;
        while (idx <4usize)
        {
            let ptr = NativePtr.OffsetMut(block.Pointer, (isize) idx);
            if (NativePtr.ReadByteMut(ptr) != 0u8)
            {
                ok = false;
            }
            idx = idx + 1;
        }
        Assert.That(ok).IsTrue();
        MemoryRuntime.chic_rt_free(block);
    }
}
