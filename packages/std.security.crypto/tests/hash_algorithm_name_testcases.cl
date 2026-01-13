namespace Std.Security.Cryptography;
import Std.Testing;

testcase Given_hash_algorithm_name_defaults_When_executed_Then_hash_algorithm_name_defaults()
{
    let name = new HashAlgorithmName(null);
    Assert.That(name.Name).IsEqualTo("");
}

testcase Given_hash_algorithm_name_factories_When_executed_Then_hash_algorithm_name_factories()
{
    let sha256 = HashAlgorithmName.Sha256();
    Assert.That(sha256.Name).IsEqualTo("SHA256");
}

testcase Given_hash_algorithm_name_factory_sha384_When_executed_Then_hash_algorithm_name_factory_sha384()
{
    let sha384 = HashAlgorithmName.Sha384();
    Assert.That(sha384.Name).IsEqualTo("SHA384");
}

testcase Given_hash_algorithm_name_factory_sha512_When_executed_Then_hash_algorithm_name_factory_sha512()
{
    let sha512 = HashAlgorithmName.Sha512();
    Assert.That(sha512.Name).IsEqualTo("SHA512");
}

testcase Given_hash_algorithm_name_equals_true_When_executed_Then_hash_algorithm_name_equals_true()
{
    let left = new HashAlgorithmName("SHA256");
    let right = new HashAlgorithmName("SHA256");
    Assert.That(left.Equals(in right)).IsTrue();
}

testcase Given_hash_algorithm_name_equals_false_When_executed_Then_hash_algorithm_name_equals_false()
{
    let left = new HashAlgorithmName("SHA256");
    let other = new HashAlgorithmName("SHA384");
    Assert.That(left.Equals(in other)).IsFalse();
}
