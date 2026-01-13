namespace Exec;

import Std.Net.Http;

public static class Program
{
    public static int Main()
    {
        var client = new HttpClient();
        var stream = client.GetStreamAsync("http://127.0.0.1:{{PORT}}/stream", default(Std.Async.CancellationToken)).Scope();
        var buffer = new byte[16];
        var span = Std.Span.Span<byte>.FromArray(ref buffer);
        var read = stream.Read(span);
        Std.Console.WriteLine(read.ToString());
        client.Dispose();
        return 0;
    }
}
