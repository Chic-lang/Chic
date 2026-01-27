namespace Std.Compiler.Lsp.Types;
public struct Hover
{
    public MarkupContent Contents;
    public bool HasRange;
    public LspRange Range;
    public init(MarkupContent contents) {
        Contents = contents;
        HasRange = false;
        Range = new LspRange(new Position(0, 0), new Position(0, 0));
    }
    public init(MarkupContent contents, LspRange range) {
        Contents = contents;
        HasRange = true;
        Range = range;
    }
}
