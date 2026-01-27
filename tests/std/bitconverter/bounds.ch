import Std;
import Std.Span;

namespace Exec;

public static class BitConverterBoundsTests
{
    public static int Main()
    {
        var tiny = Span<byte>.StackAlloc(3usize);
        if (BitConverter.TryWriteInt32(tiny, 42, Endianness.Little, out var written) || written != 0)
        {
            return 1;
        }

        var smallRead = Span<byte>.StackAlloc(4usize);
        if (BitConverter.TryReadInt64(smallRead.AsReadOnly(), Endianness.Little, out var _, out var consumed) || consumed != 0)
        {
            return 2;
        }

        var empty = Span<byte>.StackAlloc(0usize);
        if (BitConverter.TryReadBoolean(empty.AsReadOnly(), Endianness.Little, out var _, out var boolConsumed) || boolConsumed != 0)
        {
            return 3;
        }

        return 0;
    }
}
