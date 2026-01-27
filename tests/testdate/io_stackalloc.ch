import Std;
import Std.Memory;
import Std.Platform.IO;
import Std.Runtime.Collections;
import Std.Span;
import Std.Strings;

namespace Exec;

public int Main()
{
    // Validate typed stackalloc buffer handles for IO callers.
    ValueMutPtr handle = StackAlloc.Buffer<byte>(8);
    if (handle.Size != 1 || handle.Alignment != 1)
    {
        return 20;
    }

    Span<byte> bytes = StackAlloc.Span<byte>(3);
    bytes[0] = (byte)'i';
    bytes[1] = (byte)'o';
    bytes[2] = 0;
    string text = Utf8String.FromSpan(bytes.AsReadOnly().Slice(0, 2));
    if (text != "io")
    {
        return 21;
    }

    IoError status = Stdout.WriteLine("typed-io");
    return status == IoError.Success ? 0 : 22;
}
