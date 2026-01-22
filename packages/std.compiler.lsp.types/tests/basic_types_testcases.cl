namespace Std.Compiler.Lsp.Types;
import Std.Testing;

testcase Given_position_fields_When_constructed_Then_values_match()
{
    let pos = new Position(3, 7);
    Assert.That(pos.Line).IsEqualTo(3);
}

testcase Given_range_fields_When_constructed_Then_values_match()
{
    let range = new LspRange(new Position(1, 2), new Position(3, 4));
    Assert.That(range.Start.Line).IsEqualTo(1);
}

testcase Given_diagnostic_severity_error_When_used_Then_value_is_one()
{
    Assert.That((int) DiagnosticSeverity.Error).IsEqualTo(1);
}

testcase Given_initialize_result_When_constructed_Then_capabilities_match()
{
    let caps = new ServerCapabilities(TextDocumentSyncKind.Incremental, true, true);
    let result = new InitializeResult(caps, new ServerInfo("impact-lsp", "0.1.0"));
    Assert.That((int) result.Capabilities.TextDocumentSync).IsEqualTo((int) TextDocumentSyncKind.Incremental);
}
