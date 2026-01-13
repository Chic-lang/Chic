namespace Std.Security.Tls;
/// <summary>Wire content types for TLS records.</summary>
public enum TlsContentType : byte
{
    ChangeCipherSpec = 20, Alert = 21, Handshake = 22, ApplicationData = 23,
}
