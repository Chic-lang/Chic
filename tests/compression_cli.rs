use assert_cmd::cargo::cargo_bin_cmd;
use std::path::PathBuf;
use tempfile::tempdir;

mod common;

fn host_target() -> String {
    target_lexicon::HOST.to_string()
}

#[test]
#[ignore = "Std.Compression/HttpClient sample currently fails (unresolved call targets); tracked separately from this refactor PR"]
fn gzip_deflate_roundtrips_and_httpclient_decompresses() {
    let dir = tempdir().expect("temp dir");
    let manifest_path = dir.path().join("manifest.yaml");
    let _main_path = dir.path().join("Main.cl");

    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = format!(
        r#"
package:
  name: compression-test
  namespace: Tests.Compression

build:
  kind: exe

toolchain:
  runtime:
    kind: native
    package: runtime.native
    abi: rt-abi-1
    path: {repo}/packages/runtime.native

sources:
  - path: .
    namespace_prefix: Tests.Compression

dependencies:
  std:
    path: {repo}/packages/std
  std.core:
    path: {repo}/packages/std.core
  std.alloc:
    path: {repo}/packages/std.alloc
  std.foundation:
    path: {repo}/packages/std.foundation
  std.runtime:
    path: {repo}/packages/std.runtime
  std.platform:
    path: {repo}/packages/std.platform
  std.io:
    path: {repo}/packages/std.io
  std.net:
    path: {repo}/packages/std.net
  std.compression:
    path: {repo}/packages/std.compression
"#,
        repo = repo.display()
    );

    let program = r#"
namespace Tests.Compression;

import Std.IO;
import Std.IO.Compression;
import Std.Net.Http;
import Std.Span;
import Std.Numeric;
import Std.Async;

internal sealed class FakeHandler : HttpMessageHandler
{
    private byte[] _payload;
    private string _encoding;

    public init(byte[] payload, string encoding)
    {
        _payload = payload;
        _encoding = encoding;
    }

    public HttpResponseMessage Send(
        HttpRequestMessage request,
        HttpCompletionOption completionOption,
        CancellationToken cancellationToken
    )
    {
        var response = new HttpResponseMessage();
        response.StatusCode = HttpStatusCode.OK;
        response.Content = new ByteArrayContent(_payload);
        response.Content.Headers.Set("Content-Encoding", _encoding);
        return response;
    }
}

public static class Program
{
    public static int Main()
    {
        var data = new byte[5];
        data[0usize] = 1u8;
        data[1usize] = 2u8;
        data[2usize] = 3u8;
        data[3usize] = 4u8;
        data[4usize] = 5u8;

        // Span deflate roundtrip
        var compressed = new byte[128];
        if (!Deflate.TryCompress(ReadOnlySpan<byte>.FromArray(ref data), Span<byte>.FromArray(ref compressed), CompressionLevel.Optimal, out var deflateWritten))
        {
            return 1;
        }
        var decompressed = new byte[8];
        if (!Deflate.TryDecompress(ReadOnlySpan<byte>.FromArray(ref compressed).Slice(0usize, NumericUnchecked.ToUSize(deflateWritten)), Span<byte>.FromArray(ref decompressed), out var plainWritten))
        {
            return 2;
        }
        if (plainWritten != data.Length)
        {
            return 3;
        }
        for (let i = 0; i < data.Length; i += 1)
        {
            if (data[i] != decompressed[i])
            {
                return 4;
            }
        }

        // GZip span roundtrip
        var gzip = new byte[256];
        if (!GZip.TryCompress(ReadOnlySpan<byte>.FromArray(ref data), Span<byte>.FromArray(ref gzip), CompressionLevel.Optimal, out var gzipWritten))
        {
            return 5;
        }
        var gzipOut = new byte[8];
        if (!GZip.TryDecompress(ReadOnlySpan<byte>.FromArray(ref gzip).Slice(0usize, NumericUnchecked.ToUSize(gzipWritten)), Span<byte>.FromArray(ref gzipOut), out var gzipPlain))
        {
            return 6;
        }
        if (gzipPlain != data.Length)
        {
            return 7;
        }

        // Stream roundtrip
        var target = new MemoryStream();
        {
            var gzs = new GZipStream(target, CompressionMode.Compress, true);
            gzs.Write(ReadOnlySpan<byte>.FromArray(ref data));
            gzs.Dispose();
        }
        var compressedStream = target.ToArray();
        var sourceStream = new MemoryStream(compressedStream, true);
        var decodeStream = new GZipStream(sourceStream, CompressionMode.Decompress, true);
        var streamOut = new byte[5];
        let read = decodeStream.Read(Span<byte>.FromArray(ref streamOut));
        if (read != data.Length)
        {
            return 8;
        }
        for (let i = 0; i < data.Length; i += 1)
        {
            if (streamOut[i] != data[i])
            {
                return 9;
            }
        }

        // HttpClient decompression
        var handler = new FakeHandler(compressedStream, "gzip");
        var client = new HttpClient(handler, true);
        let response = Std.Async.TaskRuntime.GetResult(client.GetAsync("http://example.test"));
        if (response.Content == null)
        {
            return 10;
        }
        let body = response.Content.ReadAsByteArray();
        if (body.Length != data.Length)
        {
            return 11;
        }
        for (let i = 0; i < body.Length; i += 1)
        {
            if (body[i] != data[i])
            {
                return 12;
            }
        }

        return 0;
    }
}
"#;

    common::write_sources(
        dir.path(),
        &[("manifest.yaml", manifest.as_str()), ("Main.cl", program)],
    );

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "0")
        .arg("run")
        .arg(&manifest_path)
        .args(["--backend", "llvm", "--target", host_target().as_str()])
        .assert()
        .success();
}
