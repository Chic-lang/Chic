namespace Exec;

import Std.Net.Http;

public static class Program
{
    public static int Main()
    {
        var client = new HttpClient();
        var first = client.GetStringAsync("http://127.0.0.1:{{PORT}}/first", default(Std.Async.CancellationToken)).Scope();
        var second = client.GetStringAsync("http://127.0.0.1:{{PORT}}/second", default(Std.Async.CancellationToken)).Scope();
        Std.Console.WriteLine(first + "|" + second);
        return 0;
    }
}
