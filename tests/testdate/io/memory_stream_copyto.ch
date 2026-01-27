namespace Exec;

import Std.IO;
import Std.Span;

public static class Program
{
    public static int Main()
    {
        var source = new MemoryStream();
        var data = new byte[5];
        data[0] = 10;
        data[1] = 11;
        data[2] = 12;
        data[3] = 13;
        data[4] = 14;
        source.Write(ReadOnlySpan<byte>.FromArray(ref data));
        source.Position = 0;

        var dest = new MemoryStream();
        source.CopyTo(dest, 2);
        var copied = dest.ToArray();
        if (copied.Length != 5)
        {
            return 1;
        }
        if (copied[0] != 10 || copied[1] != 11 || copied[2] != 12 || copied[3] != 13 || copied[4] != 14)
        {
            return 2;
        }
        Std.Console.WriteLine("copy-ok");
        return 0;
    }
}
