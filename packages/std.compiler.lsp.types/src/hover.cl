namespace Std.Compiler.Lsp.Types;

public struct Hover
{
    public MarkupContent Contents;
    public bool HasRange;
    public Range Range;

    public init(MarkupContent contents) {
        Contents = contents;
        HasRange = false;
        Range = new Range(new Position(0, 0), new Position(0, 0));
    }

    public init(MarkupContent contents, Range range) {
        Contents = contents;
        HasRange = true;
        Range = range;
    }
}

