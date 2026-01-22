namespace Std.Compiler.Lsp.Types;

public struct Position
{
    public int Line;
    public int Character;

    public init(int line, int character) {
        Line = line;
        Character = character;
    }
}

