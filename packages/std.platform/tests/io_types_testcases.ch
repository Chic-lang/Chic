namespace Std.Platform.IO;
import Std;
import Std.Numeric;
import Std.Runtime.Collections;
import Std.Testing;

testcase Given_io_status_maps_errors_When_executed_Then_io_status_maps_errors()
{
    Assert.That(IoStatus.ToStatus(IoError.Success)).IsEqualTo(0);
}

testcase Given_io_status_maps_would_block_When_executed_Then_io_status_maps_would_block()
{
    Assert.That(IoStatus.ToStatus(IoError.WouldBlock)).IsEqualTo(1);
}

testcase Given_io_status_maps_eof_When_executed_Then_io_status_maps_eof()
{
    Assert.That(IoStatus.ToStatus(IoError.Eof)).IsEqualTo(6);
}

testcase Given_io_status_maps_unsupported_When_executed_Then_io_status_maps_unsupported()
{
    Assert.That(IoStatus.ToStatus(IoError.Unsupported)).IsEqualTo(10);
}

testcase Given_io_status_maps_unknown_When_executed_Then_io_status_maps_unknown()
{
    Assert.That(IoStatus.ToStatus(IoError.Unknown)).IsEqualTo(255);
}

testcase Given_io_typed_readonly_bytes_rejects_invalid_handle_When_executed_Then_io_typed_readonly_bytes_rejects_invalid_handle()
{
    unsafe {
        let ptr = PointerIntrinsics.AsByteConst(Pointer.NullConst<byte>());
        let handle = ValuePointer.CreateConst(ptr, 2usize, 2usize);
        Assert.Throws<InvalidOperationException>(() => {
            let _ = IoTyped.ReadOnlyBytes(handle, 1usize);
        });
    }
}

testcase Given_io_typed_readonly_bytes_allows_zero_length_When_executed_Then_io_typed_readonly_bytes_allows_zero_length()
{
    let handle = ValuePointer.NullConst(1usize, 1usize);
    let span = IoTyped.ReadOnlyBytes(handle, 0usize);
    Assert.That(span.Length).IsEqualTo(0usize);
}

testcase Given_io_typed_mutable_bytes_rejects_null_for_nonzero_length_When_executed_Then_io_typed_mutable_bytes_rejects_null_for_nonzero_length()
{
    let handle = ValuePointer.NullMut(1usize, 1usize);
    Assert.Throws<InvalidOperationException>(() => {
        let _ = IoTyped.MutableBytes(handle, 1usize);
    });
}

testcase Given_io_typed_from_string_status_maps_codes_When_executed_Then_io_typed_from_string_status_maps_codes()
{
    Assert.That(IoTyped.FromStringStatus(0)).IsEqualTo(IoError.Success);
}

testcase Given_io_typed_from_string_status_invalid_pointer_When_executed_Then_io_typed_from_string_status_invalid_pointer()
{
    Assert.That(IoTyped.FromStringStatus(4)).IsEqualTo(IoError.InvalidPointer);
}

testcase Given_io_typed_from_string_status_invalid_data_5_When_executed_Then_io_typed_from_string_status_invalid_data_5()
{
    Assert.That(IoTyped.FromStringStatus(5)).IsEqualTo(IoError.InvalidData);
}

testcase Given_io_typed_from_string_status_invalid_data_1_When_executed_Then_io_typed_from_string_status_invalid_data_1()
{
    Assert.That(IoTyped.FromStringStatus(1)).IsEqualTo(IoError.InvalidData);
}

testcase Given_io_typed_from_string_status_unknown_3_When_executed_Then_io_typed_from_string_status_unknown_3()
{
    Assert.That(IoTyped.FromStringStatus(3)).IsEqualTo(IoError.Unknown);
}

testcase Given_io_typed_from_string_status_unknown_2_When_executed_Then_io_typed_from_string_status_unknown_2()
{
    Assert.That(IoTyped.FromStringStatus(2)).IsEqualTo(IoError.Unknown);
}

testcase Given_io_typed_from_string_status_unknown_default_When_executed_Then_io_typed_from_string_status_unknown_default()
{
    Assert.That(IoTyped.FromStringStatus(99)).IsEqualTo(IoError.Unknown);
}
