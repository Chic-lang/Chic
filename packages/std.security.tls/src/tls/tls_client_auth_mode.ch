namespace Std.Security.Tls;
/// <summary>Controls whether a TLS server requests/validates client certificates.</summary>
public enum TlsClientAuthMode
{
    None = 0, Optional = 1, Required = 2,
}
