namespace Std.Compiler.Lsp.Types;
public struct DidChangeTextDocumentParams
{
    public VersionedTextDocumentIdentifier TextDocument;
    public TextDocumentContentChangeEvent[] ContentChanges;
    public init(VersionedTextDocumentIdentifier textDocument, TextDocumentContentChangeEvent[] contentChanges) {
        TextDocument = textDocument;
        ContentChanges = contentChanges;
    }
}
