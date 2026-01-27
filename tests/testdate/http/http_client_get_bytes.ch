namespace Exec;

import Std.Net.Http;

public static class Program
{
    public static int Main()
    {
        var client = new HttpClient();
        var bytes = client.GetByteArrayAsync("http://127.0.0.1:{{PORT}}/bytes", default(Std.Async.CancellationToken)).Scope();
        Std.Console.WriteLine(bytes.Length.ToString());
        client.Dispose();
        return 0;
    }
}
