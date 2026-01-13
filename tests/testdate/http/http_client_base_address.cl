namespace Exec;

import Std.Net.Http;

public static class Program
{
    public static int Main()
    {
        var client = new HttpClient();
        client.BaseAddress = new Std.Uri("http://127.0.0.1:{{PORT}}", Std.UriKind.Absolute);
        var response = client.GetAsync("/base", default(Std.Async.CancellationToken)).Scope();
        var body = response.Content.ReadAsString();
        Std.Console.WriteLine(body);
        client.Dispose();
        return 0;
    }
}
