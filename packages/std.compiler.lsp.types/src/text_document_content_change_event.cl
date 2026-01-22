namespace Std.Compiler.Lsp.Types;

public struct TextDocumentContentChangeEvent
{
    public bool HasRange;
    public Range Range;
    public string Text;

    public init(string text) {
        HasRange = false;
        Range = new Range(new Position(0, 0), new Position(0, 0));
        Text = text;
    }

    public init(Range range, string text) {
        HasRange = true;
        Range = range;
        Text = text;
    }
}

