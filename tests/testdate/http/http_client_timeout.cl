namespace Exec;

import Std.Net.Http;

public static class Program
{
    public static int Main()
    {
        var client = new HttpClient();
        client.Timeout = Std.Datetime.Duration.FromMilliseconds(10);
        try
        {
            var response = client.GetAsync("http://127.0.0.1:{{PORT}}/slow", default(Std.Async.CancellationToken)).Scope();
            var _ = response.Content.ReadAsString();
            return 1;
        }
        catch (Std.TaskCanceledException)
        {
            Std.Console.WriteLine("timeout");
            return 0;
        }
    }
}
