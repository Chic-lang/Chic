namespace Exec;

import Std.Net;
import Std.Async;

public static class Program
{
    public static int Main()
    {
        var addresses = Dns.GetHostAddresses("127.0.0.1");
        if (addresses.Length == 0)
        {
            return 1;
        }
        if (!addresses[0].IsIPv4)
        {
            return 2;
        }
        return 0;
    }
}
