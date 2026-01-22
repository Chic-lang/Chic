namespace Std.Compiler.Lsp.Types;

public struct ServerCapabilities
{
    public TextDocumentSyncKind TextDocumentSync;
    public bool HoverProvider;
    public bool DefinitionProvider;

    public init(TextDocumentSyncKind syncKind, bool hoverProvider, bool definitionProvider) {
        TextDocumentSync = syncKind;
        HoverProvider = hoverProvider;
        DefinitionProvider = definitionProvider;
    }
}

