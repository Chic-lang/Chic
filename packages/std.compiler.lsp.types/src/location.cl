namespace Std.Compiler.Lsp.Types;

public struct Location
{
    public string Uri;
    public LspRange Range;

    public init(string uri, LspRange range) {
        Uri = uri;
        Range = range;
    }
}
