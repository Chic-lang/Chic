namespace Std.Security.Tls;
/// <summary>Configuration for TLS clients.</summary>
public sealed class TlsClientOptions
{
    public string ServerName;
    public TlsProtocol[] EnabledProtocols;
    public string[] TrustedRootFiles;
    public bool CheckRevocation;
    public bool AllowUntrustedCertificates;
    public string[] ApplicationProtocols;
    public init() {
        ServerName = "";
        EnabledProtocols = DefaultProtocols();
        TrustedRootFiles = new string[0];
        CheckRevocation = false;
        AllowUntrustedCertificates = false;
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
