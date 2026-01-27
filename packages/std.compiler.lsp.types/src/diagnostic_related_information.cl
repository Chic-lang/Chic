namespace Std.Compiler.Lsp.Types;
public struct DiagnosticRelatedInformation
{
    public Location Location;
    public string Message;
    public init(Location location, string message) {
        Location = location;
        Message = message;
    }
}
