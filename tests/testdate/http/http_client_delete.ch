namespace Exec;

import Std.Net.Http;

public static class Program
{
    public static int Main()
    {
        var client = new HttpClient();
        var response = client.DeleteAsync("http://127.0.0.1:{{PORT}}/delete", default(Std.Async.CancellationToken)).Scope();
        var body = response.Content.ReadAsString();
        Std.Console.WriteLine(body);
        client.Dispose();
        return 0;
    }
}
