namespace Std.Compiler.Lsp.Types;

public struct TextDocumentContentChangeEvent
{
    public bool HasRange;
    public LspRange Range;
    public string Text;

    public init(string text) {
        HasRange = false;
        Range = new LspRange(new Position(0, 0), new Position(0, 0));
        Text = text;
    }

    public init(LspRange range, string text) {
        HasRange = true;
        Range = range;
        Text = text;
    }
}
