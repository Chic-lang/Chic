namespace Exec;

import Std.IO;
import Std.Span;

public static class Program
{
    public static int Main()
    {
        var ms = new MemoryStream();
        var bytes = new byte[3];
        bytes[0] = 1;
        bytes[1] = 2;
        bytes[2] = 3;
        ms.Write(ReadOnlySpan<byte>.FromArray(ref bytes));
        ms.Position = 1;
        ms.WriteByte(9);
        ms.Position = 0;
        var buffer = new byte[3];
        var span = Span<byte>.FromArray(ref buffer);
        ms.Read(span);
        if (buffer[0] != 1 || buffer[1] != 9 || buffer[2] != 3)
        {
            return 1;
        }
        ms.SetLength(2);
        if (ms.Length != 2)
        {
            return 2;
        }
        ms.Position = 2;
        var extra = new byte[2];
        extra[0] = 7;
        extra[1] = 8;
        ms.Write(ReadOnlySpan<byte>.FromArray(ref extra));
        if (ms.Length != 4)
        {
            return 3;
        }
        var all = ms.ToArray();
        Std.Console.WriteLine(all.Length.ToString());
        Std.Console.WriteLine(all[0].ToString() + "," + all[1].ToString() + "," + all[2].ToString() + "," + all[3].ToString());
        return 0;
    }
}
