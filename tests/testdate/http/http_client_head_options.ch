namespace Exec;

import Std.Net.Http;

public static class Program
{
    public static int Main()
    {
        var client = new HttpClient();
        var head = client.HeadAsync("http://127.0.0.1:{{PORT}}/info", default(Std.Async.CancellationToken)).Scope();
        var options = client.OptionsAsync("http://127.0.0.1:{{PORT}}/opts", default(Std.Async.CancellationToken)).Scope();

        var headValue = head.Headers.TryGetValue("x-head", out var headerText) ? headerText : "missing";
        var body = options.Content.ReadAsString();
        Std.Console.WriteLine(headValue + "|" + body);
        return 0;
    }
}
