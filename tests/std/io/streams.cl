namespace Exec;

import Std;
import Std.IO;
import Std.Numeric;
import Std.Span;

testcase MemoryStream_ReadWriteSeek()
{
    var ms = new MemoryStream();
    var data = new byte[] { 1, 2, 3, 4 };
    ms.Write(ReadOnlySpan<byte>.FromArray(ref data));
    ms.Position = 0;

    var buffer = new byte[4];
    let read = ms.Read(Span<byte>.FromArray(ref buffer));
    if (read != 4 || !Matches(buffer, data))
    {
        ms.Dispose();
        return false;
    }

    // Seek beyond current length and write a single byte; gap should be zero-filled.
    ms.Position = 6;
    var single = new byte[] { 9 };
    ms.Write(ReadOnlySpan<byte>.FromArray(ref single));
    if (ms.Length != 7)
    {
        ms.Dispose();
        return false;
    }
    let array = ms.ToArray();
    ms.Dispose();
    if (array.Length != 7 || array[4] != 0u8 || array[5] != 0u8 || array[6] != 9u8)
    {
        return false;
    }
    return true;
}

testcase MemoryStream_CapacityExpose()
{
    var ms = new MemoryStream();
    var initial = new byte[] { 1, 2 };
    ms.Write(ReadOnlySpan<byte>.FromArray(ref initial));
    ms.Capacity = 8;
    if (ms.Capacity != 8)
    {
        ms.Dispose();
        return false;
    }
    if (!ms.TryGetBuffer(out var exposed) || exposed.Length != 8)
    {
        ms.Dispose();
        return false;
    }
    ms.Dispose();
    return true;
}

testcase Stream_Dispose_PreventsUsage()
{
    var ms = new MemoryStream();
    ms.Dispose();
    var buffer = new byte[1];
    try
    {
        ms.Read(Span<byte>.FromArray(ref buffer));
    }
    catch (Std.ObjectDisposedException)
    {
        return true;
    }
    return false;
}

testcase FileStream_RoundTrip()
{
    var path = "io_test_" + Std.Uuid.NewUuid().ToString();
    var fs = new FileStream(path, FileMode.Create, FileAccess.ReadWrite, FileShare.Read);
    var data = new byte[] { 1, 2, 3, 4, 5 };
    fs.Write(ReadOnlySpan<byte>.FromArray(ref data));
    fs.Flush();
    fs.Position = 0;

    var readBuffer = new byte[5];
    let read = fs.Read(Span<byte>.FromArray(ref readBuffer));
    fs.Dispose();
    if (read != 5 || !Matches(readBuffer, data))
    {
        return false;
    }

    var append = new FileStream(path, FileMode.Append, FileAccess.Write, FileShare.Read);
    var more = new byte[] { 9 };
    append.Write(ReadOnlySpan<byte>.FromArray(ref more));
    append.Dispose();

    var reopen = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read);
    var total = new byte[6];
    let totalRead = reopen.Read(Span<byte>.FromArray(ref total));
    reopen.Dispose();

    if (totalRead != 6 || !Matches(total, new byte[] { 1, 2, 3, 4, 5, 9 }))
    {
        return false;
    }
    return true;
}

private static bool Matches(byte[] actual, byte[] expected)
{
    if (actual.Length != expected.Length)
    {
        return false;
    }
    var span = ReadOnlySpan<byte>.FromArray(ref actual);
    let expectedSpan = ReadOnlySpan<byte>.FromArray(ref expected);
    return Matches(span, expectedSpan);
}

private static bool Matches(ReadOnlySpan<byte> actual, ReadOnlySpan<byte> expected)
{
    if (actual.Length != expected.Length)
    {
        return false;
    }
    for (var i = 0usize; i < actual.Length; i++)
    {
        if (actual[i] != expected[i])
        {
            return false;
        }
    }
    return true;
}

public int Main()
{
    return 0;
}
