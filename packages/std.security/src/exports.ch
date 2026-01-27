namespace Std.Security;
import Std;
import Std.Testing;
/// <summary>
/// Compatibility shim that aggregates the split Std.Security packages.
/// Prefer referencing Std.Security.Cryptography, Std.Security.Tls, or Std.Security.Certs directly.
/// </summary>
public static class SecurityPackage
{
}

testcase Given_security_package_type_id_is_nonzero_When_executed_Then_security_package_type_id_is_nonzero()
{
    let typeId = Std.Type.Of<SecurityPackage>().Id;
    Assert.That(typeId).IsNotEqualTo(0ul);
}
