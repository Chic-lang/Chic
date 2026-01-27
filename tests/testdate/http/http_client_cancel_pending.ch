namespace Exec;

import Std.Net.Http;

public static class Program
{
    public static int Main()
    {
        var client = new HttpClient();
        client.CancelPendingRequests();
        try
        {
            var response = client.GetAsync("http://127.0.0.1:{{PORT}}/cancel", default(Std.Async.CancellationToken)).Scope();
            var _ = response.Content.ReadAsString();
            return 1;
        }
        catch (Std.TaskCanceledException)
        {
            Std.Console.WriteLine("canceled");
            return 0;
        }
    }
}
