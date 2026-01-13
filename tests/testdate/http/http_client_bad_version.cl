namespace Exec;

import Std.Net.Http;

public static class Program
{
    public static int Main()
    {
        var client = new HttpClient();
        try
        {
            var request = new HttpRequestMessage(HttpMethod.Get, new Std.Uri("http://127.0.0.1:{{PORT}}/"));
            request.Version = new Std.Version(2, 0);
            request.VersionPolicy = HttpVersionPolicy.RequestVersionExact;
            var response = client.Send(request);
            return response == null ? 0 : 1;
        }
        catch (HttpRequestException)
        {
            return 0;
        }
    }
}
