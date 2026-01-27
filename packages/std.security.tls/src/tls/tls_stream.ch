namespace Std.Security.Tls;
import Std.Async;
import Std.Core;
import Std.IO;
import Std.Span;
import Std.Numeric;
import Std.Security.Cryptography;
import Std.Security.Certs;
/// <summary>Native TLS stream built on top of Std.IO.Stream.</summary>
public sealed class TlsStream : Std.IO.Stream
{
    private const int RandomSize = 32;
    private const int KeySize128 = 16;
    private const int KeySize256 = 32;
    private const int IvSize = 12;
    private const int HandshakeHeaderSize = 2;
    private readonly Stream _inner;
    private readonly bool _leaveInnerOpen;
    private bool _authenticated;
    private bool _disposed;
    private TlsRecordAead ?_encryptor;
    private TlsRecordAead ?_decryptor;
    private ulong _writeSeq;
    private ulong _readSeq;
    private byte[] ?_readCache;
    private int _readCacheOffset;
    private int _readCacheLength;
    private TlsProtocol _protocol;
    private TlsCipherSuite _cipherSuite;
    private string _negotiatedAlpn;
    public init(Stream inner, bool leaveInnerOpen = false) {
        if (inner == null)
        {
            throw new Std.ArgumentNullException("inner");
        }
        _inner = inner;
        _leaveInnerOpen = leaveInnerOpen;
        _authenticated = false;
        _disposed = false;
        _writeSeq = 0ul;
        _readSeq = 0ul;
        _readCache = null;
        _readCacheOffset = 0;
        _readCacheLength = 0;
        _protocol = TlsProtocol.Tls13;
        _cipherSuite = TlsCipherSuite.TlsAes128GcmSha256;
        _negotiatedAlpn = "";
    }
    public TlsProtocol NegotiatedProtocol => _protocol;
    public TlsCipherSuite NegotiatedCipherSuite => _cipherSuite;
    public string ApplicationProtocol => _negotiatedAlpn;
    public Task AuthenticateAsClientAsync(TlsClientOptions options, CancellationToken ct) {
        EnsureNotDisposed();
        CheckCanceled(ct);
        if (options == null)
        {
            throw new Std.ArgumentNullException("options");
        }
        PerformClientHandshake(options, ct);
        _authenticated = true;
        return TaskRuntime.CompletedTask();
    }
    public Task AuthenticateAsServerAsync(TlsServerOptions options, CancellationToken ct) {
        EnsureNotDisposed();
        CheckCanceled(ct);
        if (options == null)
        {
            throw new Std.ArgumentNullException("options");
        }
        PerformServerHandshake(options, ct);
        _authenticated = true;
        return TaskRuntime.CompletedTask();
    }
    public override bool CanRead => ! _disposed && _inner.CanRead;
    public override bool CanWrite => ! _disposed && _inner.CanWrite;
    public override bool CanSeek => false;
    public override long Length => throw new Std.NotSupportedException("TlsStream does not expose Length");
    public override long Position {
        get {
            throw new Std.NotSupportedException("TlsStream does not expose Position");
        }
        set {
            throw new Std.NotSupportedException("TlsStream does not expose Position");
        }
    }
    public override int Read(Span <byte >buffer) {
        EnsureReady();
        if (buffer.Length == 0)
        {
            return 0;
        }
        if (_readCacheLength >0 && _readCache != null)
        {
            return DrainCache(buffer);
        }
        var header = Span <byte >.StackAlloc(5usize);
        let headerRead = _inner.Read(header);
        if (headerRead == 0)
        {
            return 0;
        }
        if (headerRead <5)
        {
            throw new TlsProtocolException("incomplete TLS record header");
        }
        let length = NumericUnchecked.ToUInt16((NumericUnchecked.ToUInt32(header[3]) << 8) | NumericUnchecked.ToUInt32(header[4]));
        var recordBuffer = new byte[NumericUnchecked.ToInt32(5u16 + length)];
        var recordSpan = Span <byte >.FromArray(ref recordBuffer);
        recordSpan.Slice(0usize, 5usize).CopyFrom(header);
        if (length >0u16)
        {
            let remaining = NumericUnchecked.ToInt32(length);
            let read = _inner.Read(recordSpan.Slice(5usize, NumericUnchecked.ToUSize(remaining)));
            if (read <remaining)
            {
                throw new TlsProtocolException("incomplete TLS record payload");
            }
        }
        var plaintext = new byte[NumericUnchecked.ToInt32(length)];
        let written = _decryptor.DecryptRecord(_readSeq, recordSpan.AsReadOnly(), Span <byte >.FromArray(ref plaintext),
        out var contentType);
        _readSeq += 1ul;
        if (contentType != TlsContentType.ApplicationData)
        {
            throw new TlsProtocolException("unexpected TLS content type");
        }
        if (written <= buffer.Length)
        {
            buffer.Slice(0usize, NumericUnchecked.ToUSize(written)).CopyFrom(ReadOnlySpan <byte >.FromArray(ref plaintext).Slice(0usize,
            NumericUnchecked.ToUSize(written)));
            return written;
        }
        // Cache the remainder for the next read.
        if (_readCache == null || _readCache.Length <written)
        {
            _readCache = new byte[written];
        }
        let cacheSpan = Span <byte >.FromArray(ref _readCache);
        cacheSpan.Slice(0usize, NumericUnchecked.ToUSize(written)).CopyFrom(ReadOnlySpan <byte >.FromArray(ref plaintext).Slice(0usize,
        NumericUnchecked.ToUSize(written)));
        _readCacheOffset = buffer.Length;
        _readCacheLength = written;
        buffer.CopyFrom(ReadOnlySpan <byte >.FromArray(ref _readCache).Slice(0usize, NumericUnchecked.ToUSize(buffer.Length)));
        return buffer.Length;
    }
    public override void Write(ReadOnlySpan <byte >buffer) {
        EnsureReady();
        if (buffer.Length == 0)
        {
            return;
        }
        let recordSize = 5usize + buffer.Length + 16usize;
        var record = new byte[NumericUnchecked.ToInt32(recordSize)];
        let span = Span <byte >.FromArray(ref record);
        let written = _encryptor.EncryptRecord(_writeSeq, TlsContentType.ApplicationData, buffer, span);
        _writeSeq += 1ul;
        _inner.Write(span.Slice(0usize, NumericUnchecked.ToUSize(written)));
    }
    public override void Flush() {
        EnsureReady();
        _inner.Flush();
    }
    public override long Seek(long offset, SeekOrigin origin) {
        throw new Std.NotSupportedException("TlsStream does not support seeking");
    }
    public override void SetLength(long value) {
        throw new Std.NotSupportedException("TlsStream does not support SetLength");
    }
    public override void Dispose() {
        Dispose(true);
    }
    protected override void Dispose(bool disposing) {
        if (_disposed)
        {
            return;
        }
        _disposed = true;
        if (disposing && ! _leaveInnerOpen)
        {
            _inner.Dispose();
        }
        base.Dispose(disposing);
    }
    private void PerformClientHandshake(TlsClientOptions options, CancellationToken ct) {
        var clientRandom = RandomNumberGenerator.GetBytes(RandomSize);
        var keyPair = X25519.GenerateKeyPair();
        let protocolMask = BuildProtocolMask(options.EnabledProtocols);
        let cipherMask = BuildCipherMask();
        let sni = options.ServerName != null ?options.ServerName.AsUtf8Span() : ReadOnlySpan <byte >.Empty;
        let alpnLength = AlpnLength(options.ApplicationProtocols);
        let helloLength = 1usize
        // type
        + 1usize
        // protocols
        + 1usize
        // cipher mask
        + RandomSize + X25519.PublicKeySize + 2usize
        // sni length
        + sni.Length + 1usize
        // ALPN count
        + alpnLength;
        var payload = new byte[helloLength];
        var span = Span <byte >.FromArray(ref payload);
        var offset = 0usize;
        payload[NumericUnchecked.ToInt32(offset)] = (byte) TlsHandshakeType.ClientHello;
        offset += 1usize;
        payload[NumericUnchecked.ToInt32(offset)] = protocolMask;
        offset += 1usize;
        payload[NumericUnchecked.ToInt32(offset)] = cipherMask;
        offset += 1usize;
        span.Slice(offset, RandomSize).CopyFrom(ReadOnlySpan <byte >.FromArray(ref clientRandom));
        offset += RandomSize;
        span.Slice(offset, X25519.PublicKeySize).CopyFrom(ReadOnlySpan <byte >.FromArray(ref keyPair.PublicKey));
        offset += X25519.PublicKeySize;
        WriteUInt16(NumericUnchecked.ToUInt16(sni.Length), span.Slice(offset, 2usize));
        offset += 2usize;
        if (sni.Length >0usize)
        {
            span.Slice(offset, sni.Length).CopyFrom(sni);
            offset += sni.Length;
        }
        offset = WriteAlpn(options.ApplicationProtocols, payload, offset);
        SendHandshake(span.Slice(0usize, offset));
        let serverHello = ReceiveHandshake(ct);
        ParseServerHello(serverHello, ReadOnlySpan <byte >.FromArray(ref keyPair.PrivateKey), ReadOnlySpan <byte >.FromArray(ref clientRandom),
        options);
    }
    private void PerformServerHandshake(TlsServerOptions options, CancellationToken ct) {
        let clientHello = ReceiveHandshake(ct);
        let parsed = ParseClientHello(clientHello, options);
        var serverRandom = RandomNumberGenerator.GetBytes(RandomSize);
        var keyPair = X25519.GenerateKeyPair();
        var shared = new byte[X25519.SharedSecretSize];
        let computed = X25519.ComputeSharedSecret(ReadOnlySpan <byte >.FromArray(ref keyPair.PrivateKey), parsed.ClientPublicKey.AsReadOnly(),
        Span <byte >.FromArray(ref shared));
        if (computed != X25519.SharedSecretSize)
        {
            throw new TlsHandshakeException("failed to derive shared secret");
        }
        ConfigureProtectors(parsed.CipherSuite, ReadOnlySpan <byte >.FromArray(ref shared), parsed.ClientRandom.AsReadOnly(),
        ReadOnlySpan <byte >.FromArray(ref serverRandom), false);
        _protocol = parsed.Protocol;
        _negotiatedAlpn = parsed.SelectedAlpn;
        var serverNameText = options.ServerName;
        if (serverNameText == null)
        {
            serverNameText = "";
        }
        var helloLength = 1usize
        // type
        + 1usize
        // protocol
        + 1usize
        // cipher
        + RandomSize + X25519.PublicKeySize + 2usize
        // certificate length
        + options.CertificateChain.Length + 2usize
        // server name length
        + serverNameText.Length + 1usize
        // alpn length
        + _negotiatedAlpn.Length;
        var payload = new byte[helloLength];
        var span = Span <byte >.FromArray(ref payload);
        var offset = 0usize;
        payload[NumericUnchecked.ToInt32(offset)] = (byte) TlsHandshakeType.ServerHello;
        offset += 1usize;
        payload[NumericUnchecked.ToInt32(offset)] = (byte) parsed.Protocol;
        offset += 1usize;
        payload[NumericUnchecked.ToInt32(offset)] = (byte) parsed.CipherSuite;
        offset += 1usize;
        span.Slice(offset, RandomSize).CopyFrom(ReadOnlySpan <byte >.FromArray(ref serverRandom));
        offset += RandomSize;
        span.Slice(offset, X25519.PublicKeySize).CopyFrom(ReadOnlySpan <byte >.FromArray(ref keyPair.PublicKey));
        offset += X25519.PublicKeySize;
        WriteUInt16(NumericUnchecked.ToUInt16(options.CertificateChain.Length), span.Slice(offset, 2usize));
        offset += 2usize;
        if (options.CertificateChain.Length >0)
        {
            span.Slice(offset, options.CertificateChain.Length).CopyFrom(ReadOnlySpan <byte >.FromArray(ref options.CertificateChain));
            offset += options.CertificateChain.Length;
        }
        let sni = serverNameText.Length >0 ?serverNameText.AsUtf8Span() : ReadOnlySpan <byte >.Empty;
        WriteUInt16(NumericUnchecked.ToUInt16(sni.Length), span.Slice(offset, 2usize));
        offset += 2usize;
        if (sni.Length >0usize)
        {
            span.Slice(offset, sni.Length).CopyFrom(sni);
            offset += sni.Length;
        }
        payload[NumericUnchecked.ToInt32(offset)] = NumericUnchecked.ToByte(_negotiatedAlpn.Length);
        offset += 1usize;
        if (_negotiatedAlpn.Length >0)
        {
            let alpnBytes = _negotiatedAlpn.AsUtf8Span();
            span.Slice(offset, alpnBytes.Length).CopyFrom(alpnBytes);
            offset += alpnBytes.Length;
        }
        SendHandshake(span.Slice(0usize, offset));
    }
    private void ParseServerHello(ReadOnlySpan <byte >hello, ReadOnlySpan <byte >clientPrivateKey, ReadOnlySpan <byte >clientRandom,
    TlsClientOptions options) {
        var offset = 0usize;
        if (hello.Length <1usize + 1usize + 1usize + RandomSize + X25519.PublicKeySize + 2usize + 2usize + 1usize)
        {
            throw new TlsProtocolException("server hello too small");
        }
        let messageType = hello[offset];
        offset += 1usize;
        if (messageType != (byte) TlsHandshakeType.ServerHello)
        {
            throw new TlsProtocolException("unexpected handshake message");
        }
        let protocol = (TlsProtocol) hello[offset];
        offset += 1usize;
        if (! Contains (options.EnabledProtocols, protocol))
        {
            throw new TlsHandshakeException("server selected unsupported protocol");
        }
        let cipherSuite = (TlsCipherSuite) hello[offset];
        offset += 1usize;
        var serverRandom = new byte[RandomSize];
        hello.Slice(offset, RandomSize).CopyTo(Span <byte >.FromArray(ref serverRandom));
        offset += RandomSize;
        var serverPub = new byte[X25519.PublicKeySize];
        hello.Slice(offset, X25519.PublicKeySize).CopyTo(Span <byte >.FromArray(ref serverPub));
        offset += X25519.PublicKeySize;
        let certLength = ReadUInt16(hello.Slice(offset, 2usize));
        offset += 2usize;
        if (offset + certLength >hello.Length)
        {
            throw new TlsProtocolException("invalid certificate length");
        }
        var certificate = new byte[NumericUnchecked.ToInt32(certLength)];
        if (certLength >0u16)
        {
            hello.Slice(offset, NumericUnchecked.ToUSize(certLength)).CopyTo(Span <byte >.FromArray(ref certificate));
        }
        offset += NumericUnchecked.ToUSize(certLength);
        let nameLength = ReadUInt16(hello.Slice(offset, 2usize));
        offset += 2usize;
        if (offset + nameLength >hello.Length)
        {
            throw new TlsProtocolException("invalid server name length");
        }
        var serverName = "";
        if (nameLength >0u16)
        {
            var nameBytes = hello.Slice(offset, NumericUnchecked.ToUSize(nameLength));
            serverName = Utf8String.FromSpan(nameBytes);
        }
        offset += NumericUnchecked.ToUSize(nameLength);
        var alpnLength = hello[offset];
        offset += 1usize;
        var alpn = "";
        if (alpnLength >0u8)
        {
            if (offset + alpnLength >hello.Length)
            {
                throw new TlsProtocolException("invalid ALPN length");
            }
            var alpnBytes = hello.Slice(offset, NumericUnchecked.ToUSize(alpnLength));
            alpn = Utf8String.FromSpan(alpnBytes);
        }
        var shared = new byte[X25519.SharedSecretSize];
        let written = X25519.ComputeSharedSecret(clientPrivateKey, ReadOnlySpan <byte >.FromArray(ref serverPub), Span <byte >.FromArray(ref shared));
        if (written != X25519.SharedSecretSize)
        {
            throw new TlsHandshakeException("failed to derive shared secret");
        }
        ConfigureProtectors(cipherSuite, ReadOnlySpan <byte >.FromArray(ref shared), clientRandom, ReadOnlySpan <byte >.FromArray(ref serverRandom),
        true);
        _protocol = protocol;
        _negotiatedAlpn = alpn;
        ValidateCertificate(certificate, serverName, options);
    }
    private ClientHello ParseClientHello(ReadOnlySpan <byte >hello, TlsServerOptions options) {
        var offset = 0usize;
        if (hello.Length <1usize + 1usize + 1usize + RandomSize + X25519.PublicKeySize + 2usize + 1usize)
        {
            throw new TlsProtocolException("client hello too small");
        }
        let type = hello[offset];
        offset += 1usize;
        if (type != (byte) TlsHandshakeType.ClientHello) {
            throw new TlsProtocolException("unexpected handshake message");
        }
        let protocolMask = hello[offset];
        offset += 1usize;
        let cipherMask = hello[offset];
        offset += 1usize;
        var clientRandom = new byte[RandomSize];
        hello.Slice(offset, RandomSize).CopyTo(Span <byte >.FromArray(ref clientRandom));
        offset += RandomSize;
        var clientPub = new byte[X25519.PublicKeySize];
        hello.Slice(offset, X25519.PublicKeySize).CopyTo(Span <byte >.FromArray(ref clientPub));
        offset += X25519.PublicKeySize;
        let sniLength = ReadUInt16(hello.Slice(offset, 2usize));
        offset += 2usize;
        var sni = "";
        if (sniLength >0u16)
        {
            if (offset + sniLength >hello.Length)
            {
                throw new TlsProtocolException("invalid SNI length");
            }
            var sniBytes = hello.Slice(offset, NumericUnchecked.ToUSize(sniLength));
            sni = Utf8String.FromSpan(sniBytes);
        }
        offset += NumericUnchecked.ToUSize(sniLength);
        let alpnCount = hello[offset];
        offset += 1usize;
        var selectedAlpn = "";
        var idx = 0usize;
        while (idx <NumericUnchecked.ToUSize (alpnCount))
        {
            if (offset >= hello.Length)
            {
                break;
            }
            let length = hello[offset];
            offset += 1usize;
            if (offset + length >hello.Length)
            {
                break;
            }
            var protoBytes = hello.Slice(offset, NumericUnchecked.ToUSize(length));
            let proto = Utf8String.FromSpan(protoBytes);
            if (selectedAlpn.Length == 0 && Contains (options.ApplicationProtocols, proto))
            {
                selectedAlpn = proto;
            }
            offset += NumericUnchecked.ToUSize(length);
            idx += 1usize;
        }
        let protocol = SelectProtocol(protocolMask, options.EnabledProtocols);
        let cipher = SelectCipher(cipherMask);
        var fallbackAlpn = "";
        if (options.ApplicationProtocols != null && options.ApplicationProtocols.Length >0)
        {
            fallbackAlpn = options.ApplicationProtocols[0];
        }
        return new ClientHello(protocol, cipher, clientRandom, clientPub, sni, selectedAlpn.Length >0 ?selectedAlpn : fallbackAlpn);
    }
    private void ValidateCertificate(byte[] certificate, string serverName, TlsClientOptions options) {
        let expectedHost = options.ServerName;
        if (expectedHost == null)
        {
            expectedHost = "";
        }
        let presented = serverName;
        if (! CertificateValidator.MatchesHost (expectedHost, presented))
        {
            throw new TlsCertificateException("hostname mismatch");
        }
        if (options.AllowUntrustedCertificates)
        {
            return;
        }
        let span = ReadOnlySpan <byte >.FromArray(ref certificate);
        if (! CertificateValidator.IsTrusted (span, options.TrustedRootFiles))
        {
            throw new TlsCertificateException("untrusted server certificate");
        }
    }
    private void ConfigureProtectors(TlsCipherSuite suite, ReadOnlySpan <byte >sharedSecret, ReadOnlySpan <byte >clientRandom,
    ReadOnlySpan <byte >serverRandom, bool actingAsClient) {
        let keySize = suite == TlsCipherSuite.TlsAes256GcmSha384 ?KeySize256 : KeySize128;
        let hash = suite == TlsCipherSuite.TlsAes256GcmSha384 ?HashAlgorithmName.SHA384 : HashAlgorithmName.SHA256;
        let totalSize = (keySize + IvSize) * 2usize;
        var salt = new byte[RandomSize * 2];
        var saltSpan = Span <byte >.FromArray(ref salt);
        saltSpan.Slice(0usize, RandomSize).CopyFrom(clientRandom);
        saltSpan.Slice(RandomSize, RandomSize).CopyFrom(serverRandom);
        var infoText = "chic-tls";
        let infoBytes = infoText.AsUtf8Span();
        var material = HKDF.DeriveKey(hash, sharedSecret, ReadOnlySpan <byte >.FromArray(ref salt), infoBytes, NumericUnchecked.ToInt32(totalSize));
        var offset = 0usize;
        var clientKey = new byte[keySize];
        ReadOnlySpan <byte >.FromArray(ref material).Slice(offset, NumericUnchecked.ToUSize(keySize)).CopyTo(Span <byte >.FromArray(ref clientKey));
        offset += NumericUnchecked.ToUSize(keySize);
        var clientIv = new byte[IvSize];
        ReadOnlySpan <byte >.FromArray(ref material).Slice(offset, IvSize).CopyTo(Span <byte >.FromArray(ref clientIv));
        offset += IvSize;
        var serverKey = new byte[keySize];
        ReadOnlySpan <byte >.FromArray(ref material).Slice(offset, NumericUnchecked.ToUSize(keySize)).CopyTo(Span <byte >.FromArray(ref serverKey));
        offset += NumericUnchecked.ToUSize(keySize);
        var serverIv = new byte[IvSize];
        ReadOnlySpan <byte >.FromArray(ref material).Slice(offset, IvSize).CopyTo(Span <byte >.FromArray(ref serverIv));
        if (actingAsClient)
        {
            _encryptor = new TlsRecordAead(suite, ReadOnlySpan <byte >.FromArray(ref clientKey), ReadOnlySpan <byte >.FromArray(ref clientIv));
            _decryptor = new TlsRecordAead(suite, ReadOnlySpan <byte >.FromArray(ref serverKey), ReadOnlySpan <byte >.FromArray(ref serverIv));
        }
        else
        {
            _encryptor = new TlsRecordAead(suite, ReadOnlySpan <byte >.FromArray(ref serverKey), ReadOnlySpan <byte >.FromArray(ref serverIv));
            _decryptor = new TlsRecordAead(suite, ReadOnlySpan <byte >.FromArray(ref clientKey), ReadOnlySpan <byte >.FromArray(ref clientIv));
        }
        _cipherSuite = suite;
    }
    private void SendHandshake(ReadOnlySpan <byte >payload) {
        var length = NumericUnchecked.ToUInt16(payload.Length);
        var header = Span <byte >.StackAlloc(HandshakeHeaderSize);
        WriteUInt16(length, header);
        var buffer = new byte[HandshakeHeaderSize + payload.Length];
        var span = Span <byte >.FromArray(ref buffer);
        span.Slice(0usize, HandshakeHeaderSize).CopyFrom(header);
        span.Slice(HandshakeHeaderSize, payload.Length).CopyFrom(payload);
        _inner.Write(span);
        _inner.Flush();
    }
    private ReadOnlySpan <byte >ReceiveHandshake(CancellationToken ct) {
        var header = Span <byte >.StackAlloc(HandshakeHeaderSize);
        ReadExact(header, ct);
        let length = ReadUInt16(header);
        if (length == 0u16)
        {
            throw new TlsProtocolException("handshake message missing payload");
        }
        var payload = new byte[length];
        ReadExact(Span <byte >.FromArray(ref payload), ct);
        return ReadOnlySpan <byte >.FromArray(ref payload);
    }
    private void ReadExact(Span <byte >destination, CancellationToken ct) {
        var offset = 0usize;
        while (offset <destination.Length)
        {
            CheckCanceled(ct);
            let read = _inner.Read(destination.Slice(offset, destination.Length - offset));
            if (read == 0)
            {
                throw new TlsProtocolException("unexpected end of stream during handshake");
            }
            offset += NumericUnchecked.ToUSize(read);
        }
    }
    private int DrainCache(Span <byte >buffer) {
        if (_readCache == null || _readCacheLength == 0)
        {
            return 0;
        }
        let available = _readCacheLength - _readCacheOffset;
        let toCopy = available;
        if (toCopy >buffer.Length)
        {
            toCopy = buffer.Length;
        }
        let source = ReadOnlySpan <byte >.FromArray(ref _readCache).Slice(NumericUnchecked.ToUSize(_readCacheOffset), NumericUnchecked.ToUSize(toCopy));
        buffer.Slice(0usize, NumericUnchecked.ToUSize(toCopy)).CopyFrom(source);
        _readCacheOffset += toCopy;
        if (_readCacheOffset >= _readCacheLength)
        {
            _readCacheOffset = 0;
            _readCacheLength = 0;
        }
        return toCopy;
    }
    private static void WriteUInt16(ushort value, Span <byte >destination) {
        destination[0] = NumericUnchecked.ToByte((value >> 8) & 0xFFu16);
        destination[1usize] = NumericUnchecked.ToByte(value & 0xFFu16);
    }
    private static ushort ReadUInt16(ReadOnlySpan <byte >buffer) {
        return NumericUnchecked.ToUInt16((NumericUnchecked.ToUInt32(buffer[0]) << 8) | NumericUnchecked.ToUInt32(buffer[1usize]));
    }
    private static byte BuildProtocolMask(TlsProtocol[] protocols) {
        var mask = 0u8;
        if (protocols != null)
        {
            var idx = 0usize;
            while (idx <protocols.Length)
            {
                if (protocols[idx] == TlsProtocol.Tls12)
                {
                    mask = (byte)(mask | 0x01u8);
                }
                if (protocols[idx] == TlsProtocol.Tls13)
                {
                    mask = (byte)(mask | 0x02u8);
                }
                idx += 1usize;
            }
        }
        return mask;
    }
    private static byte BuildCipherMask() {
        // Support both AES-GCM suites by default.
        return 0x03u8;
    }
    private static bool Contains(TlsProtocol[] protocols, TlsProtocol target) {
        if (protocols == null)
        {
            return false;
        }
        var idx = 0usize;
        while (idx <protocols.Length)
        {
            if (protocols[idx] == target)
            {
                return true;
            }
            idx += 1usize;
        }
        return false;
    }
    private static bool Contains(string[] values, string target) {
        if (values == null || values.Length == 0)
        {
            return false;
        }
        var idx = 0usize;
        while (idx <values.Length)
        {
            if (values[idx] == target)
            {
                return true;
            }
            idx += 1usize;
        }
        return false;
    }
    private static TlsProtocol SelectProtocol(byte mask, TlsProtocol[] serverSupported) {
        // Prefer TLS 1.3 then 1.2.
        if ( (mask & 0x02u8) != 0u8 && Contains (serverSupported, TlsProtocol.Tls13))
        {
            return TlsProtocol.Tls13;
        }
        if ( (mask & 0x01u8) != 0u8 && Contains (serverSupported, TlsProtocol.Tls12))
        {
            return TlsProtocol.Tls12;
        }
        throw new TlsHandshakeException("no common TLS protocol");
    }
    private static TlsCipherSuite SelectCipher(byte mask) {
        if ( (mask & 0x01u8) != 0u8)
        {
            return TlsCipherSuite.TlsAes128GcmSha256;
        }
        if ( (mask & 0x02u8) != 0u8)
        {
            return TlsCipherSuite.TlsAes256GcmSha384;
        }
        throw new TlsHandshakeException("no common cipher suite");
    }
    private static usize AlpnLength(string[] protocols) {
        if (protocols == null)
        {
            return 0usize;
        }
        var total = 0usize;
        var idx = 0usize;
        while (idx <protocols.Length)
        {
            let proto = protocols[idx];
            if (proto != null)
            {
                let bytes = proto.AsUtf8Span();
                total += 1usize + bytes.Length;
            }
            idx += 1usize;
        }
        return total;
    }
    private static usize WriteAlpn(string[] protocols, byte[] destination, usize offset) {
        var span = Span <byte >.FromArray(ref destination);
        if (protocols == null)
        {
            destination[NumericUnchecked.ToInt32(offset)] = 0u8;
            return offset + 1usize;
        }
        if (protocols.Length >255)
        {
            throw new Std.ArgumentException("too many ALPN entries");
        }
        destination[NumericUnchecked.ToInt32(offset)] = NumericUnchecked.ToByte(protocols.Length);
        offset += 1usize;
        var idx = 0usize;
        while (idx <protocols.Length)
        {
            var proto = protocols[idx];
            if (proto == null)
            {
                proto = "";
            }
            let bytes = proto.AsUtf8Span();
            if (bytes.Length >255usize)
            {
                throw new Std.ArgumentException("ALPN entry too long");
            }
            destination[NumericUnchecked.ToInt32(offset)] = NumericUnchecked.ToByte(bytes.Length);
            offset += 1usize;
            span.Slice(offset, bytes.Length).CopyFrom(bytes);
            offset += bytes.Length;
            idx += 1usize;
        }
        return offset;
    }
    private static void CheckCanceled(CancellationToken token) {
        if (token.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("TLS operation canceled");
        }
    }
    private void EnsureReady() {
        EnsureNotDisposed();
        if (! _authenticated)
        {
            throw new TlsHandshakeException("TLS handshake not completed");
        }
    }
    private void EnsureNotDisposed() {
        if (_disposed)
        {
            throw new Std.ObjectDisposedException("TlsStream has been disposed");
        }
    }
    internal struct ClientHello
    {
        public TlsProtocol Protocol;
        public TlsCipherSuite CipherSuite;
        public byte[] ClientRandom;
        public byte[] ClientPublicKey;
        public string ServerName;
        public string SelectedAlpn;
        public init(TlsProtocol protocol, TlsCipherSuite cipher, byte[] clientRandom, byte[] clientPublicKey, string serverName,
        string alpn) {
            Protocol = protocol;
            CipherSuite = cipher;
            ClientRandom = clientRandom;
            ClientPublicKey = clientPublicKey;
            ServerName = serverName;
            SelectedAlpn = alpn;
        }
    }
}
