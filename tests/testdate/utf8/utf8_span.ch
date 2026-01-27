import Std;
import Std.Span;
import Std.Strings;

namespace Exec;

public int Main()
{
    // Validate AsUtf8Span + TryCopyUtf8 round-trips through typed spans.
    ReadOnlySpan<byte> hello = "hello".AsUtf8Span();
    if (hello.Length != 5 || hello[0] != (byte)'h')
    {
        return 10;
    }

    Span<byte> scratch = Span<byte>.StackAlloc(6);
    if (!"hello".TryCopyUtf8(scratch, out var written) || written != 5)
    {
        return 11;
    }
    ReadOnlySpan<byte> trimmed = scratch.AsReadOnly().Slice(0, written);
    if (Utf8String.FromSpan(trimmed) != "hello")
    {
        return 12;
    }

    // Ensure Span<byte> callers route through the typed read-only entrypoint.
    Span<byte> bytes = Span<byte>.StackAlloc(4);
    bytes[0] = (byte)'d';
    bytes[1] = (byte)'a';
    bytes[2] = (byte)'t';
    bytes[3] = (byte)'a';
    if (Utf8String.FromSpan(bytes.AsReadOnly()) != "data")
    {
        return 13;
    }

    return 0;
}
