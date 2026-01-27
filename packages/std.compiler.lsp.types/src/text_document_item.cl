namespace Std.Compiler.Lsp.Types;
public struct TextDocumentItem
{
    public string Uri;
    public string LanguageId;
    public int Version;
    public string Text;
    public init(string uri, string languageId, int version, string text) {
        Uri = uri;
        LanguageId = languageId;
        Version = version;
        Text = text;
    }
}
