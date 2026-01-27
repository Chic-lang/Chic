namespace Std.Security.Tls;
import Std.Testing;
testcase Given_tls_client_options_defaults_When_executed_Then_tls_client_options_defaults()
{
    let options = new TlsClientOptions();
    Assert.That(options.ServerName).IsEqualTo("");
}
testcase Given_tls_client_options_protocol_count_When_executed_Then_tls_client_options_protocol_count()
{
    let options = new TlsClientOptions();
    Assert.That(options.EnabledProtocols.Length).IsEqualTo(2usize);
}
testcase Given_tls_client_options_protocol_first_When_executed_Then_tls_client_options_protocol_first()
{
    let options = new TlsClientOptions();
    Assert.That(options.EnabledProtocols[0]).IsEqualTo(TlsProtocol.Tls13);
}
testcase Given_tls_client_options_protocol_second_When_executed_Then_tls_client_options_protocol_second()
{
    let options = new TlsClientOptions();
    Assert.That(options.EnabledProtocols[1]).IsEqualTo(TlsProtocol.Tls12);
}
testcase Given_tls_client_options_trusted_roots_empty_When_executed_Then_tls_client_options_trusted_roots_empty()
{
    let options = new TlsClientOptions();
    Assert.That(options.TrustedRootFiles.Length).IsEqualTo(0usize);
}
testcase Given_tls_client_options_check_revocation_false_When_executed_Then_tls_client_options_check_revocation_false()
{
    let options = new TlsClientOptions();
    Assert.That(options.CheckRevocation).IsFalse();
}
testcase Given_tls_client_options_allow_untrusted_false_When_executed_Then_tls_client_options_allow_untrusted_false()
{
    let options = new TlsClientOptions();
    Assert.That(options.AllowUntrustedCertificates).IsFalse();
}
testcase Given_tls_client_options_application_protocol_default_When_executed_Then_tls_client_options_application_protocol_default()
{
    let options = new TlsClientOptions();
    Assert.That(options.ApplicationProtocols[0]).IsEqualTo("http/1.1");
}
testcase Given_tls_server_options_protocol_count_When_executed_Then_tls_server_options_protocol_count()
{
    let options = new TlsServerOptions();
    Assert.That(options.EnabledProtocols.Length).IsEqualTo(2usize);
}
testcase Given_tls_server_options_protocol_first_When_executed_Then_tls_server_options_protocol_first()
{
    let options = new TlsServerOptions();
    Assert.That(options.EnabledProtocols[0]).IsEqualTo(TlsProtocol.Tls13);
}
testcase Given_tls_server_options_protocol_second_When_executed_Then_tls_server_options_protocol_second()
{
    let options = new TlsServerOptions();
    Assert.That(options.EnabledProtocols[1]).IsEqualTo(TlsProtocol.Tls12);
}
testcase Given_tls_server_options_client_auth_default_When_executed_Then_tls_server_options_client_auth_default()
{
    let options = new TlsServerOptions();
    Assert.That(options.ClientAuthentication).IsEqualTo(TlsClientAuthMode.None);
}
testcase Given_tls_server_options_certificate_chain_empty_When_executed_Then_tls_server_options_certificate_chain_empty()
{
    let options = new TlsServerOptions();
    Assert.That(options.CertificateChain.Length).IsEqualTo(0usize);
}
testcase Given_tls_server_options_private_key_empty_When_executed_Then_tls_server_options_private_key_empty()
{
    let options = new TlsServerOptions();
    Assert.That(options.PrivateKey.Length).IsEqualTo(0usize);
}
testcase Given_tls_server_options_server_name_empty_When_executed_Then_tls_server_options_server_name_empty()
{
    let options = new TlsServerOptions();
    Assert.That(options.ServerName).IsEqualTo("");
}
testcase Given_tls_server_options_application_protocol_default_When_executed_Then_tls_server_options_application_protocol_default()
{
    let options = new TlsServerOptions();
    Assert.That(options.ApplicationProtocols[0]).IsEqualTo("http/1.1");
}
testcase Given_tls_exception_message_When_executed_Then_tls_exception_message()
{
    let baseEx = new TlsException("tls");
    Assert.That(baseEx.Message).IsEqualTo("tls");
}
testcase Given_tls_handshake_exception_message_When_executed_Then_tls_handshake_exception_message()
{
    let handshake = new TlsHandshakeException("handshake");
    Assert.That(handshake.Message).IsEqualTo("handshake");
}
testcase Given_tls_alert_exception_message_When_executed_Then_tls_alert_exception_message()
{
    let alert = new TlsAlertException("alert");
    Assert.That(alert.Message).IsEqualTo("alert");
}
testcase Given_tls_certificate_exception_message_When_executed_Then_tls_certificate_exception_message()
{
    let cert = new TlsCertificateException("cert");
    Assert.That(cert.Message).IsEqualTo("cert");
}
testcase Given_tls_protocol_exception_message_When_executed_Then_tls_protocol_exception_message()
{
    let protocol = new TlsProtocolException("proto");
    Assert.That(protocol.Message).IsEqualTo("proto");
}
