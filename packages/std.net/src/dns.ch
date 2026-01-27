namespace Std.Net;
/// <summary>Minimal DNS helper stub.</summary>
public static class Dns
{
    public static IPAddress[] GetHostAddresses(string host) {
        let loopback = IPAddress.Parse("127.0.0.1");
        return new IPAddress[1usize] {
            loopback
        }
        ;
    }
}
