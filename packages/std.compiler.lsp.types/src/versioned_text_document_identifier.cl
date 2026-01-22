namespace Std.Compiler.Lsp.Types;

public struct VersionedTextDocumentIdentifier
{
    public string Uri;
    public int Version;

    public init(string uri, int version) {
        Uri = uri;
        Version = version;
    }
}

