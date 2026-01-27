namespace Std.Security.Tls;
/// <summary>Configuration for TLS servers.</summary>
public sealed class TlsServerOptions
{
    public TlsProtocol[] EnabledProtocols;
    public TlsClientAuthMode ClientAuthentication;
    public byte[] CertificateChain;
    public byte[] PrivateKey;
    public string ServerName;
    public string[] ApplicationProtocols;
    public init() {
        EnabledProtocols = DefaultProtocols();
        ClientAuthentication = TlsClientAuthMode.None;
        CertificateChain = new byte[0];
        PrivateKey = new byte[0];
        ServerName = "";
        ApplicationProtocols = DefaultAlpn();
    }
    private static TlsProtocol[] DefaultProtocols() {
        var protocols = new TlsProtocol[2];
        protocols[0] = TlsProtocol.Tls13;
        protocols[1] = TlsProtocol.Tls12;
        return protocols;
    }
    private static string[] DefaultAlpn() {
        var protocols = new string[1];
        protocols[0] = "http/1.1";
        return protocols;
    }
}
