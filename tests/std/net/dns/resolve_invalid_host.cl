namespace Exec;

import Std.Net;

public static class Program
{
    public static int Main()
    {
        try
        {
            var _ = Dns.GetHostAddresses("invalid.invalid.invalid");
            return 1; // should throw
        }
        catch (Std.NotSupportedException)
        {
            return 0;
        }
    }
}
