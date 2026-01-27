namespace Exec;

import Std.Net.Http;

public static class Program
{
    public static int Main()
    {
        var client = new HttpClient();
        try
        {
            var response = client.GetAsync("http://127.0.0.1:{{PORT}}/short", default(Std.Async.CancellationToken)).Scope();
            var _ = response.Content.ReadAsString();
            return 1;
        }
        catch (HttpRequestException)
        {
            Std.Console.WriteLine("incomplete");
            return 0;
        }
    }
}
