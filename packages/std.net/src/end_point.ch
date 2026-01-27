namespace Std.Net;
/// <summary>Base endpoint abstraction.</summary>
public abstract class EndPoint
{
    public abstract AddressFamily AddressFamily {
        get;
    }
}
