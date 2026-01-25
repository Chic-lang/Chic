namespace Std.Compiler.Lsp.Types;
public struct Diagnostic
{
    public LspRange Range;
    public DiagnosticSeverity Severity;
    public bool HasCode;
    public string Code;
    public string Source;
    public string Message;
    public DiagnosticRelatedInformation[] RelatedInformation;
    public init(LspRange range, DiagnosticSeverity severity, string source, string message) {
        Range = range;
        Severity = severity;
        HasCode = false;
        Code = "";
        Source = source;
        Message = message;
        RelatedInformation = null;
    }
}
