namespace Exec;

import Std.Net.Http;
import Std.Net.Http.Json;

public static class Program
{
    public static int Main()
    {
        var client = new HttpClient();
        var payload = "ping";
        var response = client.PostAsJsonAsync("http://127.0.0.1:{{PORT}}/json", payload, default(Std.Async.CancellationToken)).Scope();
        var body = response.Content.ReadAsString();
        Std.Console.WriteLine(body);
        client.Dispose();
        return 0;
    }
}
