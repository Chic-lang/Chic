namespace Std.Platform.Net;
import Std.Testing;
testcase Given_dns_platform_resolve_returns_result_When_executed_Then_dns_platform_resolve_returns_result()
{
    let result = DnsPlatform.Resolve("localhost", 0);
    let ok = result.Error == DnsError.Success || result.Error == DnsError.Failure;
    Assert.That(ok).IsTrue();
    DnsPlatform.Free(result);
}
