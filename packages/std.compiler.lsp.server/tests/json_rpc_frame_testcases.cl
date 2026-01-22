namespace Std.Compiler.Lsp.Server;
import Std.Testing;

testcase Given_json_rpc_frame_When_serialized_Then_wire_contains_content_length()
{
    let frame = new JsonRpcFrame("{\"jsonrpc\":\"2.0\"}");
    let wire = frame.ToWire();
    Assert.That(wire.StartsWith("Content-Length:")).IsTrue();
}

testcase Given_json_rpc_frame_When_roundtripped_Then_body_matches()
{
    let frame = new JsonRpcFrame("{\"jsonrpc\":\"2.0\"}");
    let wire = frame.ToWire();
    let parsed = JsonRpcFrame.TryParse(wire, out var decoded);
    Assert.That(parsed).IsTrue();
}

testcase Given_json_rpc_frame_parsed_When_roundtripped_Then_body_matches()
{
    let frame = new JsonRpcFrame("{\"jsonrpc\":\"2.0\"}");
    let wire = frame.ToWire();
    let _ = JsonRpcFrame.TryParse(wire, out var decoded);
    Assert.That(decoded.Body).IsEqualTo("{\"jsonrpc\":\"2.0\"}");
}

