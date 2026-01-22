namespace Std.Compiler.Lsp.Types;

public struct TextDocumentIdentifier
{
    public string Uri;

    public init(string uri) {
        Uri = uri;
    }
}

