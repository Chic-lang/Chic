namespace Std.Security.Tls;
/// <summary>Internal TLS handshake message types used by TlsStream.</summary>
public enum TlsHandshakeType : byte
{
    ClientHello = 1, ServerHello = 2, Finished = 20,
}
