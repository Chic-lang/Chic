namespace Exec;

import Std.Net.Http;

public static class Program
{
    public static int Main()
    {
        var client = new HttpClient();
        var response = client.GetAsync("http://127.0.0.1:{{PORT}}/hello").Scope();
        var body = response.Content.ReadAsString();
        Std.Console.WriteLine(body);
        client.Dispose();
        return 0;
    }
}
