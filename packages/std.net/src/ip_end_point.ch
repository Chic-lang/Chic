namespace Std.Net;
/// <summary>Represents a network endpoint with IP address and port.</summary>
public class IPEndPoint : EndPoint
{
    private IPAddress _address;
    private int _port;
    public init(IPAddress address, int port) {
        if (port <0 || port >65535)
        {
            throw new Std.ArgumentOutOfRangeException("port");
        }
        _address = address;
        _port = port;
    }
    public IPAddress Address {
        get {
            return _address;
        }
        set {
            _address = value;
        }
    }
    public int Port {
        get {
            return _port;
        }
        set {
            if (value <0 || value >65535)
            {
                throw new Std.ArgumentOutOfRangeException("port");
            }
            _port = value;
        }
    }
    public override AddressFamily AddressFamily => _address.AddressFamily;
}
