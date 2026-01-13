namespace Std.Security.Tls;
/// <summary>Base class for TLS-related failures.</summary>
public class TlsException : Std.Exception
{
    public init() : base() {
    }
    public init(string message) : base(message) {
    }
}
/// <summary>Handshake negotiation failed.</summary>
public class TlsHandshakeException : TlsException
{
    public init() : base() {
    }
    public init(string message) : base(message) {
    }
}
/// <summary>Alert or record-level failure.</summary>
public class TlsAlertException : TlsException
{
    public init() : base() {
    }
    public init(string message) : base(message) {
    }
}
/// <summary>Certificate validation failure.</summary>
public class TlsCertificateException : TlsException
{
    public init() : base() {
    }
    public init(string message) : base(message) {
    }
}
/// <summary>Protocol violation or unexpected data.</summary>
public class TlsProtocolException : TlsException
{
    public init() : base() {
    }
    public init(string message) : base(message) {
    }
}
