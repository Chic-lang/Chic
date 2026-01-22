namespace Std.Compiler.Lsp.Types;

public struct Location
{
    public string Uri;
    public Range Range;

    public init(string uri, Range range) {
        Uri = uri;
        Range = range;
    }
}

