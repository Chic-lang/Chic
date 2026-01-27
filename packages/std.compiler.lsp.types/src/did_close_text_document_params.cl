namespace Std.Compiler.Lsp.Types;
public struct DidCloseTextDocumentParams
{
    public TextDocumentIdentifier TextDocument;
    public init(TextDocumentIdentifier textDocument) {
        TextDocument = textDocument;
    }
}
