namespace Std.Security.Certs;
import Std;
import Std.Testing;

testcase Given_certificate_is_trusted_requires_roots_When_executed_Then_certificate_is_trusted_requires_roots()
{
    let cert = Std.Span.ReadOnlySpan.FromString("cert");
    let ok = CertificateValidator.IsTrusted(cert, new string[0]);
    Assert.That(ok).IsFalse();
}

testcase Given_certificate_matches_host_normalizes_case_When_executed_Then_certificate_matches_host_normalizes_case()
{
    let ok = CertificateValidator.MatchesHost("Example.COM", "example.com");
    Assert.That(ok).IsTrue();
}

testcase Given_certificate_matches_host_rejects_empty_presented_When_executed_Then_certificate_matches_host_rejects_empty_presented()
{
    let ok = CertificateValidator.MatchesHost("example.com", "");
    Assert.That(ok).IsFalse();
}
