namespace Std.Net.Http;
import Std;
import Std.Async;
import Std.Core;
import Std.Datetime;
import Std.Security.Tls;
import Std.Testing;

private sealed class ExposedHttpClientHandler : HttpClientHandler
{
    public Task<HttpResponseMessage> SendAsyncPublic(HttpRequestMessage request, HttpCompletionOption completionOption, CancellationToken token)
    {
        return SendAsync(request, completionOption, token);
    }
}

testcase Given_http_client_handler_defaults_When_executed_Then_http_client_handler_defaults()
{
    var handler = new HttpClientHandler();
    Assert.That(handler.Timeout.Ticks).IsEqualTo(Duration.FromSeconds(100).Ticks);
}

testcase Given_http_client_handler_default_max_response_buffer_When_executed_Then_http_client_handler_default_max_response_buffer()
{
    var handler = new HttpClientHandler();
    Assert.That(handler.MaxResponseContentBufferSize).IsEqualTo(1024 * 1024);
}

testcase Given_http_client_handler_default_allow_untrusted_false_When_executed_Then_http_client_handler_default_allow_untrusted_false()
{
    var handler = new HttpClientHandler();
    Assert.That(handler.AllowUntrustedCertificates).IsFalse();
}

testcase Given_http_client_handler_default_trusted_roots_empty_When_executed_Then_http_client_handler_default_trusted_roots_empty()
{
    var handler = new HttpClientHandler();
    Assert.That(handler.TrustedRootCertificates.Length).IsEqualTo(0);
}

testcase Given_http_client_handler_default_protocols_length_When_executed_Then_http_client_handler_default_protocols_length()
{
    var handler = new HttpClientHandler();
    Assert.That(handler.EnabledProtocols.Length).IsEqualTo(2);
}

testcase Given_http_client_handler_default_protocol_first_When_executed_Then_http_client_handler_default_protocol_first()
{
    var handler = new HttpClientHandler();
    Assert.That(handler.EnabledProtocols[0]).IsEqualTo(TlsProtocol.Tls13);
}

testcase Given_http_client_handler_default_protocol_second_When_executed_Then_http_client_handler_default_protocol_second()
{
    var handler = new HttpClientHandler();
    Assert.That(handler.EnabledProtocols[1]).IsEqualTo(TlsProtocol.Tls12);
}

testcase Given_http_client_handler_default_app_protocols_length_When_executed_Then_http_client_handler_default_app_protocols_length()
{
    var handler = new HttpClientHandler();
    Assert.That(handler.ApplicationProtocols.Length).IsEqualTo(1);
}

testcase Given_http_client_handler_default_app_protocol_When_executed_Then_http_client_handler_default_app_protocol()
{
    var handler = new HttpClientHandler();
    Assert.That(handler.ApplicationProtocols[0]).IsEqualTo("http/1.1");
}

testcase Given_http_client_handler_send_async_rejects_null_request_When_executed_Then_http_client_handler_send_async_rejects_null_request()
{
    var handler = new ExposedHttpClientHandler();
    let token = CoreIntrinsics.DefaultValue<CancellationToken>();
    Assert.Throws<HttpRequestException>(() => {
        let _ = handler.SendAsyncPublic(null, HttpCompletionOption.ResponseContentRead, token);
    });
}

testcase Given_http_client_handler_send_async_rejects_missing_uri_When_executed_Then_http_client_handler_send_async_rejects_missing_uri()
{
    var handler = new ExposedHttpClientHandler();
    var request = new HttpRequestMessage();
    let token = CoreIntrinsics.DefaultValue<CancellationToken>();
    Assert.Throws<HttpRequestException>(() => {
        let _ = handler.SendAsyncPublic(request, HttpCompletionOption.ResponseContentRead, token);
    });
}

testcase Given_http_client_handler_rejects_http2_exact_When_executed_Then_http_client_handler_rejects_http2_exact()
{
    var handler = new ExposedHttpClientHandler();
    var request = new HttpRequestMessage(HttpMethod.Get, new Std.Uri("https://example.com"));
    request.Version = new Std.Version(2, 0);
    request.VersionPolicy = HttpVersionPolicy.RequestVersionExact;
    let token = CoreIntrinsics.DefaultValue<CancellationToken>();
    Assert.Throws<HttpRequestException>(() => {
        let _ = handler.SendAsyncPublic(request, HttpCompletionOption.ResponseContentRead, token);
    });
}
