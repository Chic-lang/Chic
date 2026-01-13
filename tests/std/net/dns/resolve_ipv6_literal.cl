namespace Exec;

import Std.Net;

public static class Program
{
    public static int Main()
    {
        var addresses = Dns.GetHostAddresses("::1");
        if (addresses.Length == 0)
        {
            return 1;
        }
        if (!addresses[0].IsIPv6)
        {
            return 2;
        }
        return 0;
    }
}
