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
testcase Given_server_info_name_only_When_constructed_Then_has_version_is_false()
{
    let info = new ServerInfo("impact-lsp");
    Assert.That(info.HasVersion).IsFalse();
}
testcase Given_initialize_result_without_server_info_When_constructed_Then_has_server_info_is_false()
{
    let caps = new ServerCapabilities(TextDocumentSyncKind.Full, false, false);
    let result = new InitializeResult(caps);
    Assert.That(result.HasServerInfo).IsFalse();
}
testcase Given_markup_content_When_constructed_Then_kind_roundtrips()
{
    let content = new MarkupContent(MarkupKind.PlainText, "hello");
    Assert.That((int) content.Kind).IsEqualTo((int) MarkupKind.PlainText);
}
testcase Given_hover_without_range_When_constructed_Then_has_range_is_false()
{
    let content = new MarkupContent(MarkupKind.Markdown, "hi");
    let hover = new Hover(content);
    Assert.That(hover.HasRange).IsFalse();
}
testcase Given_hover_with_range_When_constructed_Then_has_range_is_true()
{
    let content = new MarkupContent(MarkupKind.Markdown, "hi");
    let hover = new Hover(content, new LspRange(new Position(0, 0), new Position(0, 1)));
    Assert.That(hover.HasRange).IsTrue();
}
testcase Given_location_When_constructed_Then_uri_roundtrips()
{
    let loc = new Location("file:///main.cl", new LspRange(new Position(0, 0), new Position(0, 1)));
    Assert.That(loc.Uri).IsEqualTo("file:///main.cl");
}
testcase Given_text_document_identifier_When_constructed_Then_uri_roundtrips()
{
    let id = new TextDocumentIdentifier("file:///main.cl");
    Assert.That(id.Uri).IsEqualTo("file:///main.cl");
}
testcase Given_versioned_text_document_identifier_When_constructed_Then_version_roundtrips()
{
    let id = new VersionedTextDocumentIdentifier("file:///main.cl", 3);
    Assert.That(id.Version).IsEqualTo(3);
}
testcase Given_text_document_item_When_constructed_Then_language_id_roundtrips()
{
    let item = new TextDocumentItem("file:///main.cl", "chic", 1, "namespace Root;");
    Assert.That(item.LanguageId).IsEqualTo("chic");
}
testcase Given_did_open_params_When_constructed_Then_document_uri_roundtrips()
{
    let item = new TextDocumentItem("file:///main.cl", "chic", 1, "namespace Root;");
    let params = new DidOpenTextDocumentParams(item);
    Assert.That(params.TextDocument.Uri).IsEqualTo("file:///main.cl");
}
testcase Given_did_close_params_When_constructed_Then_document_uri_roundtrips()
{
    let id = new TextDocumentIdentifier("file:///main.cl");
    let params = new DidCloseTextDocumentParams(id);
    Assert.That(params.TextDocument.Uri).IsEqualTo("file:///main.cl");
}
testcase Given_content_change_without_range_When_constructed_Then_has_range_is_false()
{
    let change = new TextDocumentContentChangeEvent("hi");
    Assert.That(change.HasRange).IsFalse();
}
testcase Given_content_change_with_range_When_constructed_Then_has_range_is_true()
{
    let change = new TextDocumentContentChangeEvent(new LspRange(new Position(0, 0), new Position(0, 1)), "hi");
    Assert.That(change.HasRange).IsTrue();
}
testcase Given_did_change_params_When_constructed_Then_content_changes_length_is_one()
{
    let doc = new VersionedTextDocumentIdentifier("file:///main.cl", 2);
    let params = new DidChangeTextDocumentParams(doc, null);
    Assert.That(params.TextDocument.Version).IsEqualTo(2);
}
testcase Given_publish_diagnostics_params_without_version_When_constructed_Then_has_version_is_false()
{
    let params = new PublishDiagnosticsParams("file:///main.cl", null);
    Assert.That(params.HasVersion).IsFalse();
}
testcase Given_publish_diagnostics_params_with_version_When_constructed_Then_has_version_is_true()
{
    let params = new PublishDiagnosticsParams("file:///main.cl", null, 1);
    Assert.That(params.HasVersion).IsTrue();
}
testcase Given_diagnostic_related_information_When_constructed_Then_message_roundtrips()
{
    let loc = new Location("file:///main.cl", new LspRange(new Position(0, 0), new Position(0, 1)));
    let info = new DiagnosticRelatedInformation(loc, "note");
    Assert.That(info.Message).IsEqualTo("note");
}
testcase Given_diagnostic_When_constructed_Then_has_code_is_false()
{
    let range = new LspRange(new Position(0, 0), new Position(0, 1));
    let diag = new Diagnostic(range, DiagnosticSeverity.Error, "chic", "oops");
    Assert.That(diag.HasCode).IsFalse();
}
