namespace Std.Net.Http;
import Std;
import Std.Async;
import Std.Core;
import Std.Datetime;
import Std.IO.Compression;
import Std.Numeric;
import Std.Span;
import Std.Testing;

private sealed class StubHandler : HttpMessageHandler
{
    public bool Disposed;
    public int SendCalls;
    public override HttpResponseMessage Send(HttpRequestMessage request, HttpCompletionOption completionOption, CancellationToken cancellationToken) {
        SendCalls += 1;
        var response = new HttpResponseMessage();
        response.StatusCode = HttpStatusCode.Accepted;
        return response;
    }
    public override void Dispose() {
        Disposed = true;
    }
}

private sealed class AsyncOnlyHandler : HttpMessageHandler
{
    public bool Called;
    public override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, HttpCompletionOption completionOption, CancellationToken cancellationToken) {
        Called = true;
        var response = new HttpResponseMessage();
        response.StatusCode = HttpStatusCode.OK;
        return TaskRuntime.FromResult(response);
    }
}

private sealed class TestInvoker : HttpMessageInvoker
{
    public init(HttpMessageHandler handler, bool disposeHandler) : base(handler, disposeHandler) {
    }
}

private sealed class CaptureHandler : HttpMessageHandler
{
    public HttpRequestMessage ?LastRequest;
    public HttpResponseMessage Response;
    public init(HttpResponseMessage response) {
        Response = response;
    }
    public override HttpResponseMessage Send(HttpRequestMessage request, HttpCompletionOption completionOption, CancellationToken cancellationToken) {
        LastRequest = request;
        return Response;
    }
}

private sealed class PassThroughHandler : DelegatingHandler
{
}

testcase Given_http_method_get_has_get_text_When_executed_Then_http_method_get_has_get_text()
{
    let get1 = HttpMethod.Get;
    Assert.That(get1.Method).IsEqualTo("GET");
}

testcase Given_http_method_get_equals_self_When_executed_Then_http_method_get_equals_self()
{
    let get1 = HttpMethod.Get;
    let get2 = HttpMethod.Get;
    Assert.That(get1.Equals(get2)).IsTrue();
}

testcase Given_http_method_custom_equals_patch_When_executed_Then_http_method_custom_equals_patch()
{
    let custom = new HttpMethod("PATCH");
    Assert.That(custom.Equals(HttpMethod.Patch)).IsTrue();
}

testcase Given_http_method_null_ctor_throws_When_executed_Then_http_method_null_ctor_throws()
{
    Assert.Throws<ArgumentNullException>(() => {
        let _ = new HttpMethod(null);
    });
}

testcase Given_http_request_message_default_method_When_executed_Then_http_request_message_default_method()
{
    var request = new HttpRequestMessage();
    Assert.That(request.Method.Method).IsEqualTo("GET");
}

testcase Given_http_request_message_default_version_major_When_executed_Then_http_request_message_default_version_major()
{
    var request = new HttpRequestMessage();
    Assert.That(request.Version.Major).IsEqualTo(1);
}

testcase Given_http_request_message_default_version_minor_When_executed_Then_http_request_message_default_version_minor()
{
    var request = new HttpRequestMessage();
    Assert.That(request.Version.Minor).IsEqualTo(1);
}

testcase Given_http_request_message_default_version_policy_When_executed_Then_http_request_message_default_version_policy()
{
    var request = new HttpRequestMessage();
    Assert.That(request.VersionPolicy).IsEqualTo(HttpVersionPolicy.RequestVersionOrLower);
}

testcase Given_http_request_message_default_headers_non_null_When_executed_Then_http_request_message_default_headers_non_null()
{
    var request = new HttpRequestMessage();
    Assert.That(request.Headers).IsNotNull();
}

testcase Given_http_request_message_with_uri_sets_method_When_executed_Then_http_request_message_with_uri_sets_method()
{
    var uri = new Std.Uri("https://example.com");
    var request = new HttpRequestMessage(HttpMethod.Post, uri);
    Assert.That(request.Method.Method).IsEqualTo("POST");
}

testcase Given_http_request_message_with_uri_sets_request_uri_When_executed_Then_http_request_message_with_uri_sets_request_uri()
{
    var uri = new Std.Uri("https://example.com");
    var request = new HttpRequestMessage(HttpMethod.Post, uri);
    Assert.That(request.RequestUri).IsNotNull();
}

testcase Given_http_response_message_default_status_ok_When_executed_Then_http_response_message_default_status_ok()
{
    var response = new HttpResponseMessage();
    Assert.That(response.StatusCode).IsEqualTo(HttpStatusCode.OK);
}

testcase Given_http_response_message_default_is_success_When_executed_Then_http_response_message_default_is_success()
{
    var response = new HttpResponseMessage();
    Assert.That(response.IsSuccessStatusCode()).IsTrue();
}

testcase Given_http_response_message_bad_request_not_success_When_executed_Then_http_response_message_bad_request_not_success()
{
    var response = new HttpResponseMessage();
    response.StatusCode = HttpStatusCode.BadRequest;
    Assert.That(response.IsSuccessStatusCode()).IsFalse();
}

testcase Given_http_request_exception_sets_status_code_When_executed_Then_http_request_exception_sets_status_code()
{
    var ex = new HttpRequestException("missing", HttpStatusCode.NotFound);
    Assert.That(ex.StatusCode).IsEqualTo(HttpStatusCode.NotFound);
}

testcase Given_http_headers_add_contains_lowercase_When_executed_Then_http_headers_add_contains_lowercase()
{
    var headers = new HttpHeaders();
    headers.Add("Content-Type", "text/plain");
    Assert.That(headers.Contains("content-type")).IsTrue();
}

testcase Given_http_headers_add_contains_original_When_executed_Then_http_headers_add_contains_original()
{
    var headers = new HttpHeaders();
    headers.Add("Content-Type", "text/plain");
    Assert.That(headers.Contains("Content-Type")).IsTrue();
}

testcase Given_http_headers_add_try_get_value_ok_When_executed_Then_http_headers_add_try_get_value_ok()
{
    var headers = new HttpHeaders();
    headers.Add("Content-Type", "text/plain");
    let ok = headers.TryGetValue("CONTENT-TYPE", out var value);
    Assert.That(ok).IsTrue();
}

testcase Given_http_headers_add_try_get_value_matches_When_executed_Then_http_headers_add_try_get_value_matches()
{
    var headers = new HttpHeaders();
    headers.Add("Content-Type", "text/plain");
    let _ = headers.TryGetValue("CONTENT-TYPE", out var value);
    Assert.That(value).IsEqualTo("text/plain");
}

testcase Given_http_headers_add_merges_values_When_executed_Then_http_headers_add_merges_values()
{
    var headers = new HttpHeaders();
    headers.Add("Content-Type", "text/plain");
    headers.Add("Content-Type", "charset=utf-8");
    let _ = headers.TryGetValue("Content-Type", out var merged);
    Assert.That(merged).IsEqualTo("text/plain, charset=utf-8");
}

testcase Given_http_headers_set_overwrites_value_When_executed_Then_http_headers_set_overwrites_value()
{
    var headers = new HttpHeaders();
    headers.Add("Content-Type", "text/plain");
    headers.Set("Content-Type", "application/json");
    let _ = headers.TryGetValue("Content-Type", out var updated);
    Assert.That(updated).IsEqualTo("application/json");
}

testcase Given_http_headers_remove_returns_true_When_executed_Then_http_headers_remove_returns_true()
{
    var headers = new HttpHeaders();
    headers.Add("Content-Type", "text/plain");
    let removed = headers.Remove("Content-Type");
    Assert.That(removed).IsTrue();
}

testcase Given_http_headers_remove_clears_contains_When_executed_Then_http_headers_remove_clears_contains()
{
    var headers = new HttpHeaders();
    headers.Add("Content-Type", "text/plain");
    let _ = headers.Remove("Content-Type");
    Assert.That(headers.Contains("Content-Type")).IsFalse();
}

testcase Given_http_headers_add_rejects_null_name_When_executed_Then_http_headers_add_rejects_null_name()
{
    var headers = new HttpHeaders();
    Assert.Throws<ArgumentNullException>(() => {
        headers.Add(null, "value");
    });
}

testcase Given_http_headers_set_rejects_null_value_When_executed_Then_http_headers_set_rejects_null_value()
{
    var headers = new HttpHeaders();
    Assert.Throws<ArgumentNullException>(() => {
        headers.Set("name", null);
    });
}

testcase Given_http_content_byte_array_roundtrip_length_When_executed_Then_http_content_byte_array_roundtrip_length()
{
    var buffer = new byte[3];
    buffer[0] = 1u8;
    buffer[1] = 2u8;
    buffer[2] = 3u8;
    var content = new ByteArrayContent(buffer);
    let bytes = content.ReadAsByteArray();
    Assert.That(bytes.Length).IsEqualTo(3);
}

testcase Given_http_content_byte_array_roundtrip_first_byte_When_executed_Then_http_content_byte_array_roundtrip_first_byte()
{
    var buffer = new byte[3];
    buffer[0] = 1u8;
    buffer[1] = 2u8;
    buffer[2] = 3u8;
    var content = new ByteArrayContent(buffer);
    let bytes = content.ReadAsByteArray();
    Assert.That(bytes[0]).IsEqualTo(1u8);
}

testcase Given_http_content_byte_array_roundtrip_header_ok_When_executed_Then_http_content_byte_array_roundtrip_header_ok()
{
    var buffer = new byte[3];
    buffer[0] = 1u8;
    buffer[1] = 2u8;
    buffer[2] = 3u8;
    var content = new ByteArrayContent(buffer);
    let ok = content.Headers.TryGetValue("Content-Length", out var lenValue);
    Assert.That(ok).IsTrue();
}

testcase Given_http_content_byte_array_roundtrip_header_value_When_executed_Then_http_content_byte_array_roundtrip_header_value()
{
    var buffer = new byte[3];
    buffer[0] = 1u8;
    buffer[1] = 2u8;
    buffer[2] = 3u8;
    var content = new ByteArrayContent(buffer);
    let _ = content.Headers.TryGetValue("Content-Length", out var lenValue);
    Assert.That(lenValue).IsEqualTo("3");
}

testcase Given_http_content_string_roundtrip_text_When_executed_Then_http_content_string_roundtrip_text()
{
    var content = new StringContent("hello");
    let text = content.ReadAsString();
    Assert.That(text).IsEqualTo("hello");
}

testcase Given_http_content_string_roundtrip_header_ok_When_executed_Then_http_content_string_roundtrip_header_ok()
{
    var content = new StringContent("hello");
    let ok = content.Headers.TryGetValue("Content-Type", out var contentType);
    Assert.That(ok).IsTrue();
}

testcase Given_http_content_string_roundtrip_header_value_When_executed_Then_http_content_string_roundtrip_header_value()
{
    var content = new StringContent("hello");
    let _ = content.Headers.TryGetValue("Content-Type", out var contentType);
    Assert.That(contentType).IsEqualTo("text/plain; charset=utf-8");
}

testcase Given_http_content_stream_roundtrip_length_When_executed_Then_http_content_stream_roundtrip_length()
{
    var buffer = new byte[2];
    buffer[0] = 9u8;
    buffer[1] = 8u8;
    var content = new StreamContent(buffer);
    let bytes = content.ReadAsByteArray();
    Assert.That(bytes.Length).IsEqualTo(2);
}

testcase Given_http_content_stream_roundtrip_second_byte_When_executed_Then_http_content_stream_roundtrip_second_byte()
{
    var buffer = new byte[2];
    buffer[0] = 9u8;
    buffer[1] = 8u8;
    var content = new StreamContent(buffer);
    let bytes = content.ReadAsByteArray();
    Assert.That(bytes[1]).IsEqualTo(8u8);
}

testcase Given_http_delegating_handler_requires_inner_handler_throws_When_executed_Then_http_delegating_handler_requires_inner_handler_throws()
{
    var handler = new PassThroughHandler();
    Assert.Throws<InvalidOperationException>(() => {
        let _ = handler.Send(new HttpRequestMessage(), HttpCompletionOption.ResponseContentRead, CoreIntrinsics.DefaultValue<CancellationToken>());
    });
}

testcase Given_http_delegating_handler_returns_response_When_executed_Then_http_delegating_handler_returns_response()
{
    var handler = new PassThroughHandler();
    var inner = new StubHandler();
    handler.InnerHandler = inner;
    let response = handler.Send(new HttpRequestMessage(), HttpCompletionOption.ResponseContentRead, CoreIntrinsics.DefaultValue<CancellationToken>());
    Assert.That(response.StatusCode).IsEqualTo(HttpStatusCode.Accepted);
}

testcase Given_http_delegating_handler_calls_inner_When_executed_Then_http_delegating_handler_calls_inner()
{
    var handler = new PassThroughHandler();
    var inner = new StubHandler();
    handler.InnerHandler = inner;
    let _ = handler.Send(new HttpRequestMessage(), HttpCompletionOption.ResponseContentRead, CoreIntrinsics.DefaultValue<CancellationToken>());
    Assert.That(inner.SendCalls).IsEqualTo(1);
}

testcase Given_http_message_invoker_disposes_handler_response_status_When_executed_Then_http_message_invoker_disposes_handler_response_status()
{
    var handler = new StubHandler();
    var invoker = new TestInvoker(handler, true);
    var response = invoker.Send(new HttpRequestMessage(), HttpCompletionOption.ResponseContentRead, CoreIntrinsics.DefaultValue<CancellationToken>());
    Assert.That(response.StatusCode).IsEqualTo(HttpStatusCode.Accepted);
}

testcase Given_http_message_invoker_disposes_handler_marks_disposed_When_executed_Then_http_message_invoker_disposes_handler_marks_disposed()
{
    var handler = new StubHandler();
    var invoker = new TestInvoker(handler, true);
    let _ = invoker.Send(new HttpRequestMessage(), HttpCompletionOption.ResponseContentRead, CoreIntrinsics.DefaultValue<CancellationToken>());
    invoker.Dispose();
    Assert.That(handler.Disposed).IsTrue();
}

testcase Given_http_message_invoker_disposes_handler_send_throws_When_executed_Then_http_message_invoker_disposes_handler_send_throws()
{
    var handler = new StubHandler();
    var invoker = new TestInvoker(handler, true);
    let _ = invoker.Send(new HttpRequestMessage(), HttpCompletionOption.ResponseContentRead, CoreIntrinsics.DefaultValue<CancellationToken>());
    invoker.Dispose();
    Assert.Throws<InvalidOperationException>(() => {
        let _ = invoker.Send(new HttpRequestMessage(), HttpCompletionOption.ResponseContentRead, CoreIntrinsics.DefaultValue<CancellationToken>());
    });
}

testcase Given_http_message_handler_send_calls_async_called_When_executed_Then_http_message_handler_send_calls_async_called()
{
    var handler = new AsyncOnlyHandler();
    let _ = handler.Send(new HttpRequestMessage(), HttpCompletionOption.ResponseContentRead, CoreIntrinsics.DefaultValue<CancellationToken>());
    Assert.That(handler.Called).IsTrue();
}

testcase Given_http_message_handler_send_calls_async_response_status_When_executed_Then_http_message_handler_send_calls_async_response_status()
{
    var handler = new AsyncOnlyHandler();
    let response = handler.Send(new HttpRequestMessage(), HttpCompletionOption.ResponseContentRead, CoreIntrinsics.DefaultValue<CancellationToken>());
    Assert.That(response.StatusCode).IsEqualTo(HttpStatusCode.OK);
}

testcase Given_http_client_prepares_relative_uri_has_request_When_executed_Then_http_client_prepares_relative_uri_has_request()
{
    var response = new HttpResponseMessage();
    var handler = new CaptureHandler(response);
    var client = new HttpClient(handler, true);
    client.BaseAddress = new Std.Uri("https://example.com/api/");
    var request = new HttpRequestMessage(HttpMethod.Get, new Std.Uri("health", Std.UriKind.Relative));
    let _ = client.Send(request);
    let prepared = handler.LastRequest;
    Assert.That(prepared).IsNotNull();
}

testcase Given_http_client_prepares_relative_uri_has_request_uri_When_executed_Then_http_client_prepares_relative_uri_has_request_uri()
{
    var response = new HttpResponseMessage();
    var handler = new CaptureHandler(response);
    var client = new HttpClient(handler, true);
    client.BaseAddress = new Std.Uri("https://example.com/api/");
    var request = new HttpRequestMessage(HttpMethod.Get, new Std.Uri("health", Std.UriKind.Relative));
    let _ = client.Send(request);
    let prepared = handler.LastRequest;
    let preparedUri = prepared == null ? null : prepared.RequestUri;
    Assert.That(preparedUri).IsNotNull();
}

testcase Given_http_client_prepares_relative_uri_is_absolute_When_executed_Then_http_client_prepares_relative_uri_is_absolute()
{
    var response = new HttpResponseMessage();
    var handler = new CaptureHandler(response);
    var client = new HttpClient(handler, true);
    client.BaseAddress = new Std.Uri("https://example.com/api/");
    var request = new HttpRequestMessage(HttpMethod.Get, new Std.Uri("health", Std.UriKind.Relative));
    let _ = client.Send(request);
    let prepared = handler.LastRequest;
    let preparedUri = prepared == null ? null : prepared.RequestUri;
    Assert.That(preparedUri.IsAbsoluteUri).IsTrue();
}

testcase Given_http_client_prepares_relative_uri_absolute_value_When_executed_Then_http_client_prepares_relative_uri_absolute_value()
{
    var response = new HttpResponseMessage();
    var handler = new CaptureHandler(response);
    var client = new HttpClient(handler, true);
    client.BaseAddress = new Std.Uri("https://example.com/api/");
    var request = new HttpRequestMessage(HttpMethod.Get, new Std.Uri("health", Std.UriKind.Relative));
    let _ = client.Send(request);
    let prepared = handler.LastRequest;
    let preparedUri = prepared == null ? null : prepared.RequestUri;
    Assert.That(preparedUri.AbsoluteUri).IsEqualTo("https://example.com/api/health");
}

testcase Given_http_client_requires_request_uri_without_base_When_executed_Then_http_client_requires_request_uri_without_base()
{
    var handler = new CaptureHandler(new HttpResponseMessage());
    var client = new HttpClient(handler, true);
    Assert.Throws<HttpRequestException>(() => {
        let _ = client.Send(new HttpRequestMessage());
    });
}

testcase Given_http_client_rejects_relative_without_base_When_executed_Then_http_client_rejects_relative_without_base()
{
    var handler = new CaptureHandler(new HttpResponseMessage());
    var client = new HttpClient(handler, true);
    var request = new HttpRequestMessage(HttpMethod.Get, new Std.Uri("relative", Std.UriKind.Relative));
    Assert.Throws<HttpRequestException>(() => {
        let _ = client.Send(request);
    });
}

testcase Given_http_client_applies_default_headers_has_request_When_executed_Then_http_client_applies_default_headers_has_request()
{
    var response = new HttpResponseMessage();
    var handler = new CaptureHandler(response);
    var client = new HttpClient(handler, true);
    client.DefaultRequestHeaders.Set("X-Test", "1");
    var request = new HttpRequestMessage(HttpMethod.Get, new Std.Uri("https://example.com"));
    let _ = client.Send(request);
    let prepared = handler.LastRequest;
    Assert.That(prepared).IsNotNull();
}

testcase Given_http_client_applies_default_headers_contains_header_When_executed_Then_http_client_applies_default_headers_contains_header()
{
    var response = new HttpResponseMessage();
    var handler = new CaptureHandler(response);
    var client = new HttpClient(handler, true);
    client.DefaultRequestHeaders.Set("X-Test", "1");
    var request = new HttpRequestMessage(HttpMethod.Get, new Std.Uri("https://example.com"));
    let _ = client.Send(request);
    let prepared = handler.LastRequest;
    Assert.That(prepared.Headers.Contains("x-test")).IsTrue();
}

testcase Given_http_client_decompresses_gzip_content_compress_ok_When_executed_Then_http_client_decompresses_gzip_content_compress_ok()
{
    let payload = ReadOnlySpan.FromString("hello");
    var compressed = new byte[64];
    let ok = GZip.TryCompress(payload, Span<byte>.FromArray(ref compressed), CompressionLevel.Optimal, out var written);
    Assert.That(ok).IsTrue();
}

testcase Given_http_client_decompresses_gzip_content_text_When_executed_Then_http_client_decompresses_gzip_content_text()
{
    let payload = ReadOnlySpan.FromString("hello");
    var compressed = new byte[64];
    let _ = GZip.TryCompress(payload, Span<byte>.FromArray(ref compressed), CompressionLevel.Optimal, out var written);
    var data = new byte[written];
    Span<byte>.FromArray(ref data).CopyFrom(ReadOnlySpan<byte>.FromArray(in compressed).Slice(0usize, NumericUnchecked.ToUSize(written)));
    var response = new HttpResponseMessage();
    response.Content = new ByteArrayContent(data);
    response.Content.Headers.Set("Content-Encoding", "gzip");
    var handler = new CaptureHandler(response);
    var client = new HttpClient(handler, true);
    var request = new HttpRequestMessage(HttpMethod.Get, new Std.Uri("https://example.com"));
    let result = client.Send(request);
    let text = result.Content.ReadAsString();
    Assert.That(text).IsEqualTo("hello");
}

testcase Given_http_client_decompresses_gzip_content_strips_header_When_executed_Then_http_client_decompresses_gzip_content_strips_header()
{
    let payload = ReadOnlySpan.FromString("hello");
    var compressed = new byte[64];
    let _ = GZip.TryCompress(payload, Span<byte>.FromArray(ref compressed), CompressionLevel.Optimal, out var written);
    var data = new byte[written];
    Span<byte>.FromArray(ref data).CopyFrom(ReadOnlySpan<byte>.FromArray(in compressed).Slice(0usize, NumericUnchecked.ToUSize(written)));
    var response = new HttpResponseMessage();
    response.Content = new ByteArrayContent(data);
    response.Content.Headers.Set("Content-Encoding", "gzip");
    var handler = new CaptureHandler(response);
    var client = new HttpClient(handler, true);
    var request = new HttpRequestMessage(HttpMethod.Get, new Std.Uri("https://example.com"));
    let result = client.Send(request);
    Assert.That(result.Content.Headers.Contains("Content-Encoding")).IsFalse();
}

testcase Given_http_client_decompresses_deflate_content_compress_ok_When_executed_Then_http_client_decompresses_deflate_content_compress_ok()
{
    let payload = ReadOnlySpan.FromString("deflate");
    var compressed = new byte[64];
    let ok = Deflate.TryCompress(payload, Span<byte>.FromArray(ref compressed), CompressionLevel.Optimal, out var written);
    Assert.That(ok).IsTrue();
}

testcase Given_http_client_decompresses_deflate_content_text_When_executed_Then_http_client_decompresses_deflate_content_text()
{
    let payload = ReadOnlySpan.FromString("deflate");
    var compressed = new byte[64];
    let _ = Deflate.TryCompress(payload, Span<byte>.FromArray(ref compressed), CompressionLevel.Optimal, out var written);
    var data = new byte[written];
    Span<byte>.FromArray(ref data).CopyFrom(ReadOnlySpan<byte>.FromArray(in compressed).Slice(0usize, NumericUnchecked.ToUSize(written)));
    var response = new HttpResponseMessage();
    response.Content = new ByteArrayContent(data);
    response.Content.Headers.Set("Content-Encoding", "deflate");
    var handler = new CaptureHandler(response);
    var client = new HttpClient(handler, true);
    var request = new HttpRequestMessage(HttpMethod.Get, new Std.Uri("https://example.com"));
    let result = client.Send(request);
    let text = result.Content.ReadAsString();
    Assert.That(text).IsEqualTo("deflate");
}

testcase Given_http_client_rejects_invalid_limits_buffer_size_When_executed_Then_http_client_rejects_invalid_limits_buffer_size()
{
    var handler = new CaptureHandler(new HttpResponseMessage());
    var client = new HttpClient(handler, true);
    Assert.Throws<ArgumentOutOfRangeException>(() => {
        client.MaxResponseContentBufferSize = -2;
    });
}

testcase Given_http_client_rejects_invalid_limits_timeout_When_executed_Then_http_client_rejects_invalid_limits_timeout()
{
    var handler = new CaptureHandler(new HttpResponseMessage());
    var client = new HttpClient(handler, true);
    Assert.Throws<ArgumentOutOfRangeException>(() => {
        client.Timeout = Duration.FromTicks(-2);
    });
}
