namespace Std.Net;
import Std;
import Std.Async;
import Std.IO;
import Std.Net.Sockets;
import Std.Span;
import Std.Testing;

private sealed class DummyEndPoint : EndPoint
{
    public override AddressFamily AddressFamily => AddressFamily.InterNetwork;
}

testcase Given_ip_address_parse_ipv4_roundtrip_When_executed_Then_ip_address_parse_ipv4_roundtrip()
{
    let addr = IPAddress.Parse("127.0.0.1");
    Assert.That(addr.IsIPv4).IsTrue();
}

testcase Given_ip_address_parse_ipv4_family_When_executed_Then_ip_address_parse_ipv4_family()
{
    let addr = IPAddress.Parse("127.0.0.1");
    Assert.That(addr.AddressFamily).IsEqualTo(AddressFamily.InterNetwork);
}

testcase Given_ip_address_parse_ipv4_to_string_When_executed_Then_ip_address_parse_ipv4_to_string()
{
    let addr = IPAddress.Parse("127.0.0.1");
    Assert.That(addr.ToString()).IsEqualTo("127.0.0.1");
}

testcase Given_ip_address_parse_ipv4_bytes_length_When_executed_Then_ip_address_parse_ipv4_bytes_length()
{
    let addr = IPAddress.Parse("127.0.0.1");
    let bytes = addr.GetAddressBytes();
    Assert.That(bytes.Length).IsEqualTo(4);
}

testcase Given_ip_address_parse_ipv4_bytes_first_When_executed_Then_ip_address_parse_ipv4_bytes_first()
{
    let addr = IPAddress.Parse("127.0.0.1");
    let bytes = addr.GetAddressBytes();
    Assert.That(bytes[0]).IsEqualTo(127u8);
}

testcase Given_ip_address_parse_ipv4_bytes_last_When_executed_Then_ip_address_parse_ipv4_bytes_last()
{
    let addr = IPAddress.Parse("127.0.0.1");
    let bytes = addr.GetAddressBytes();
    Assert.That(bytes[3]).IsEqualTo(1u8);
}

testcase Given_ip_address_try_parse_null_returns_false_When_executed_Then_ip_address_try_parse_null_returns_false()
{
    let okNull = IPAddress.TryParse(null, out var _);
    Assert.That(okNull).IsFalse();
}

testcase Given_ip_address_try_parse_bad_returns_false_When_executed_Then_ip_address_try_parse_bad_returns_false()
{
    let ok = IPAddress.TryParse("bad", out var _);
    Assert.That(ok).IsFalse();
}

testcase Given_ip_address_parse_bad_throws_When_executed_Then_ip_address_parse_bad_throws()
{
    Assert.Throws<FormatException>(() => {
        let _ = IPAddress.Parse("bad");
    });
}

testcase Given_ip_end_point_port_from_constructor_When_executed_Then_ip_end_point_port_from_constructor()
{
    let addr = IPAddress.Parse("127.0.0.1");
    var endpoint = new IPEndPoint(addr, 80);
    Assert.That(endpoint.Port).IsEqualTo(80);
}

testcase Given_ip_end_point_port_set_zero_When_executed_Then_ip_end_point_port_set_zero()
{
    let addr = IPAddress.Parse("127.0.0.1");
    var endpoint = new IPEndPoint(addr, 80);
    endpoint.Port = 0;
    Assert.That(endpoint.Port).IsEqualTo(0);
}

testcase Given_ip_end_point_ctor_invalid_port_throws_When_executed_Then_ip_end_point_ctor_invalid_port_throws()
{
    let addr = IPAddress.Parse("127.0.0.1");
    Assert.Throws<ArgumentOutOfRangeException>(() => {
        let _ = new IPEndPoint(addr, 70000);
    });
}

testcase Given_ip_end_point_set_invalid_port_throws_When_executed_Then_ip_end_point_set_invalid_port_throws()
{
    let addr = IPAddress.Parse("127.0.0.1");
    var endpoint = new IPEndPoint(addr, 80);
    Assert.Throws<ArgumentOutOfRangeException>(() => {
        endpoint.Port = -1;
    });
}

testcase Given_dns_returns_single_address_When_executed_Then_dns_returns_single_address()
{
    let results = Dns.GetHostAddresses("example.com");
    Assert.That(results.Length).IsEqualTo(1);
}

testcase Given_dns_returns_loopback_When_executed_Then_dns_returns_loopback()
{
    let results = Dns.GetHostAddresses("example.com");
    Assert.That(results[0].ToString()).IsEqualTo("127.0.0.1");
}

testcase Given_socket_connect_invalid_port_returns_invalid_When_executed_Then_socket_connect_invalid_port_returns_invalid()
{
    var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    let addr = IPAddress.Parse("127.0.0.1");
    let connectStatus = socket.Connect(addr, -1);
    Assert.That(connectStatus).IsEqualTo(SocketError.Invalid);
    socket.Close();
}

testcase Given_socket_bind_invalid_port_returns_invalid_When_executed_Then_socket_bind_invalid_port_returns_invalid()
{
    var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    let addr = IPAddress.Parse("127.0.0.1");
    let bindStatus = socket.Bind(addr, 70000);
    Assert.That(bindStatus).IsEqualTo(SocketError.Invalid);
    socket.Close();
}

testcase Given_socket_empty_send_returns_zero_When_executed_Then_socket_empty_send_returns_zero()
{
    var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    var empty = new byte[0];
    let sent = socket.Send(ReadOnlySpan<byte>.FromArray(in empty));
    Assert.That(sent).IsEqualTo(0);
    socket.Close();
}

testcase Given_socket_empty_receive_returns_zero_When_executed_Then_socket_empty_receive_returns_zero()
{
    var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    var buffer = Span<byte>.StackAlloc(0usize);
    let received = socket.Receive(buffer);
    Assert.That(received).IsEqualTo(0);
    socket.Close();
}

testcase Given_socket_send_to_throws_not_supported_When_executed_Then_socket_send_to_throws_not_supported()
{
    var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    var empty = new byte[0];
    Assert.Throws<NotSupportedException>(() => {
        let _ = socket.SendTo(ReadOnlySpan<byte>.FromArray(in empty), new DummyEndPoint());
    });
    socket.Close();
}

testcase Given_socket_connect_async_cancels_When_executed_Then_socket_connect_async_cancels()
{
    var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    var cts = CancellationTokenSource.Create();
    cts.Cancel();
    Assert.Throws<TaskCanceledException>(() => {
        let _ = socket.ConnectAsync(IPAddress.Parse("127.0.0.1"), 80, cts.Token());
    });
    socket.Close();
}

testcase Given_network_stream_can_read_true_When_executed_Then_network_stream_can_read_true()
{
    var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    var stream = new NetworkStream(socket, false);
    Assert.That(stream.CanRead).IsTrue();
    stream.Dispose();
    socket.Close();
}

testcase Given_network_stream_can_write_true_When_executed_Then_network_stream_can_write_true()
{
    var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    var stream = new NetworkStream(socket, false);
    Assert.That(stream.CanWrite).IsTrue();
    stream.Dispose();
    socket.Close();
}

testcase Given_network_stream_can_seek_false_When_executed_Then_network_stream_can_seek_false()
{
    var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    var stream = new NetworkStream(socket, false);
    Assert.That(stream.CanSeek).IsFalse();
    stream.Dispose();
    socket.Close();
}

testcase Given_network_stream_read_empty_returns_zero_When_executed_Then_network_stream_read_empty_returns_zero()
{
    var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    var stream = new NetworkStream(socket, false);
    var empty = Span<byte>.StackAlloc(0usize);
    let read = stream.Read(empty);
    Assert.That(read).IsEqualTo(0);
    stream.Dispose();
    socket.Close();
}

testcase Given_network_stream_write_empty_no_throw_When_executed_Then_network_stream_write_empty_no_throw()
{
    var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    var stream = new NetworkStream(socket, false);
    var emptyArr = new byte[0];
    stream.Write(ReadOnlySpan<byte>.FromArray(in emptyArr));
    Assert.That(true).IsTrue();
    stream.Dispose();
    socket.Close();
}

testcase Given_network_stream_length_throws_When_executed_Then_network_stream_length_throws()
{
    var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    var stream = new NetworkStream(socket, false);
    Assert.Throws<NotSupportedException>(() => {
        let _ = stream.Length;
    });
    stream.Dispose();
    socket.Close();
}

testcase Given_network_stream_position_throws_When_executed_Then_network_stream_position_throws()
{
    var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    var stream = new NetworkStream(socket, false);
    Assert.Throws<NotSupportedException>(() => {
        let _ = stream.Position;
    });
    stream.Dispose();
    socket.Close();
}

testcase Given_network_stream_seek_throws_When_executed_Then_network_stream_seek_throws()
{
    var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    var stream = new NetworkStream(socket, false);
    Assert.Throws<NotSupportedException>(() => {
        stream.Seek(0, SeekOrigin.Begin);
    });
    stream.Dispose();
    socket.Close();
}
