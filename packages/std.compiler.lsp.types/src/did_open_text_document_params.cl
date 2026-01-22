namespace Std.Compiler.Lsp.Types;

public struct DidOpenTextDocumentParams
{
    public TextDocumentItem TextDocument;

    public init(TextDocumentItem textDocument) {
        TextDocument = textDocument;
    }
}

