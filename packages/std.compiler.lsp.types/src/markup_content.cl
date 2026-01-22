namespace Std.Compiler.Lsp.Types;

public struct MarkupContent
{
    public MarkupKind Kind;
    public string Value;

    public init(MarkupKind kind, string value) {
        Kind = kind;
        Value = value;
    }
}

