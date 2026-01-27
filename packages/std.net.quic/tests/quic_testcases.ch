namespace Std.Net.Quic;
import Std;
import Std.Span;
import Std.Testing;
testcase Given_quic_connect_is_not_supported_When_executed_Then_quic_connect_is_not_supported()
{
    Assert.Throws <NotSupportedException >(() => {
        let _ = QuicConnection.Connect("localhost", 443);
    }
    );
}
testcase Given_quic_listener_is_not_supported_When_executed_Then_quic_listener_is_not_supported()
{
    Assert.Throws <NotSupportedException >(() => {
        let _ = new QuicListener(443);
    }
    );
}
testcase Given_quic_stream_read_write_throw_When_executed_Then_quic_stream_read_write_throw()
{
    var stream = new QuicStream();
    Assert.Throws <NotSupportedException >(() => {
        var buffer = Span <byte >.StackAlloc(4usize); let _ = stream.Read(buffer);
    }
    );
}
testcase Given_quic_stream_write_throws_When_executed_Then_quic_stream_write_throws()
{
    var stream = new QuicStream();
    Assert.Throws <NotSupportedException >(() => {
        let bytes = ReadOnlySpan.FromString("test"); stream.Write(bytes);
    }
    );
}
