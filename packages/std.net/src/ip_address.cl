namespace Std.Net;
import Std.Numeric;
import Std.Core;
import Std.Platform.IO;
import Std.Platform.Net;
/// <summary>Represents an IP address (IPv4-only in the bootstrap).</summary>
public struct IPAddress
{
    private uint _rawNet;
    private byte[] _rawV6;
    private bool _isV4;
    private uint _scopeId;
    private init(uint rawNet, bool isV4) {
        _rawNet = rawNet;
        _isV4 = isV4;
        _rawV6 = new byte[0usize];
        _scopeId = 0;
    }
    private init(byte[] rawV6, uint scopeId) {
        _rawNet = 0;
        _isV4 = false;
        _rawV6 = rawV6;
        _scopeId = scopeId;
    }
    internal static IPAddress FromIpv4Raw(uint rawNet) {
        return new IPAddress(rawNet, true);
    }
    internal static IPAddress FromIpv6Bytes(byte[] bytes, uint scopeId) {
        return new IPAddress(bytes, scopeId);
    }
    public static IPAddress Parse(string text) {
        if (TryParse (text, out var addr)) {
            return addr;
        }
        throw new Std.FormatException("Invalid IP address");
    }
    public static bool TryParse(string text, out IPAddress address) {
        if (text == null)
        {
            address = CoreIntrinsics.DefaultValue <IPAddress >();
            return false;
        }
        if (Ipv4Address.TryParse (text, out var v4)) {
            address = new IPAddress(v4.RawNet, true);
            return true;
        }
        if (TryParseIpv6 (text, out var v6, out var scope)) {
            address = new IPAddress(v6, scope);
            return true;
        }
        address = CoreIntrinsics.DefaultValue <IPAddress >();
        return false;
    }
    public AddressFamily AddressFamily => _isV4 ?AddressFamily.InterNetwork : AddressFamily.InterNetworkV6;
    public bool IsIPv4 => _isV4;
    public bool IsIPv6 => ! _isV4;
    public byte[] GetAddressBytes() {
        if (! _isV4)
        {
            let length = _rawV6.Length;
            var copy = new byte[length];
            if (length >0)
            {
                Span <byte >.FromArray(ref copy).CopyFrom(ReadOnlySpan <byte >.FromArray(in _rawV6));
            }
            return copy;
        }
        let v4Length = 4;
        var bytes = new byte[v4Length];
        // Stored in network order.
        bytes[0] = NumericUnchecked.ToByte(_rawNet & 0xFF);
        bytes[1] = NumericUnchecked.ToByte((_rawNet >> 8) & 0xFF);
        bytes[2] = NumericUnchecked.ToByte((_rawNet >> 16) & 0xFF);
        bytes[3] = NumericUnchecked.ToByte((_rawNet >> 24) & 0xFF);
        return bytes;
    }
    public override string ToString() {
        if (! _isV4)
        {
            if (_rawV6.Length != 16)
            {
                return "";
            }
            // Uncompressed IPv6 hex groups.
            var sb = new Std.StringWriter();
            for (var i = 0; i <8; i += 1) {
                let high = NumericUnchecked.ToUInt16(((uint) _rawV6[i * 2 + 1] << 8) | _rawV6[i * 2]);
                sb.Write(high.ToString("x"));
                if (i <7)
                {
                    sb.Write(":");
                }
            }
            let text = sb.ToString();
            sb.dispose();
            return text;
        }
        let bytes = GetAddressBytes();
        return bytes[0].ToString() + "." + bytes[1].ToString() + "." + bytes[2].ToString() + "." + bytes[3].ToString();
    }
    internal Ipv4Address ToIpv4Address() {
        var addr = CoreIntrinsics.DefaultValue <Ipv4Address >();
        addr.RawNet = _rawNet;
        return addr;
    }
    internal uint RawNet => _rawNet;
    internal ReadOnlySpan <byte >RawV6Span() {
        if (_rawV6.Length == 0)
        {
            return ReadOnlySpan <byte >.Empty;
        }
        return ReadOnlySpan <byte >.FromArray(in _rawV6);
    }
    internal uint ScopeId => _scopeId;
    private static bool TryParseIpv6(string text, out byte[] bytes, out uint scopeId) {
        var temp = text;
        let utf8 = SpanIntrinsics.chic_rt_string_as_slice(temp);
        var buf = Span <byte >.StackAlloc(utf8.len + 1);
        buf.Slice(0, utf8.len).CopyFrom(IoTyped.FromStringSlice(utf8));
        let ptr = PointerIntrinsics.AsByteConstFromMut(buf.Raw.Data.Pointer);
        var tmp = Span <byte >.StackAlloc(16);
        let status = SocketPlatform.InetPton6(ptr, tmp);
        if (status == 1)
        {
            let v6Length = 16;
            bytes = new byte[v6Length];
            Span <byte >.FromArray(ref bytes).CopyFrom(tmp.AsReadOnly());
            scopeId = 0;
            return true;
        }
        bytes = new byte[0usize];
        scopeId = 0;
        return false;
    }
}
