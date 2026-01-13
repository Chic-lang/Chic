namespace Std.Net.Quic;
/// <summary>Placeholder QUIC surface until the transport is fully implemented.</summary>
public sealed class QuicConnection
{
    public static QuicConnection Connect(string host, int port) {
        throw new Std.NotSupportedException("Std.Net.Quic is not implemented yet");
    }
    public QuicStream OpenStream() {
        throw new Std.NotSupportedException("Std.Net.Quic is not implemented yet");
    }
    public void Close() {
    }
}
public sealed class QuicListener
{
    public init(int port) {
        throw new Std.NotSupportedException("Std.Net.Quic is not implemented yet");
    }
    public QuicConnection Accept() {
        throw new Std.NotSupportedException("Std.Net.Quic is not implemented yet");
    }
    public void Close() {
    }
}
public sealed class QuicStream
{
    public int Read(Std.Span.Span <byte >destination) {
        throw new Std.NotSupportedException("Std.Net.Quic is not implemented yet");
    }
    public void Write(Std.Span.ReadOnlySpan <byte >source) {
        throw new Std.NotSupportedException("Std.Net.Quic is not implemented yet");
    }
}
