namespace Std.Compiler.Lsp.Types;
public struct PublishDiagnosticsParams
{
    public string Uri;
    public Diagnostic[] Diagnostics;
    public bool HasVersion;
    public int Version;
    public init(string uri, Diagnostic[] diagnostics) {
        Uri = uri;
        Diagnostics = diagnostics;
        HasVersion = false;
        Version = 0;
    }
    public init(string uri, Diagnostic[] diagnostics, int version) {
        Uri = uri;
        Diagnostics = diagnostics;
        HasVersion = true;
        Version = version;
    }
}
