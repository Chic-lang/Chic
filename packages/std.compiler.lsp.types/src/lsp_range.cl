namespace Std.Compiler.Lsp.Types;

public struct LspRange
{
    public Position Start;
    public Position End;

    public init(Position start, Position end) {
        Start = start;
        End = end;
    }
}
