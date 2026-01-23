namespace Std.Security.Cryptography;
import Std.Testing;
testcase Given_rng_get_bytes_zero_returns_empty_When_executed_Then_rng_get_bytes_zero_returns_empty()
{
    let data = RandomNumberGenerator.GetBytes(0);
    Assert.That(data.Length).IsEqualTo(0usize);
}
testcase Given_rng_get_bytes_negative_throws_When_executed_Then_rng_get_bytes_negative_throws()
{
    Assert.Throws <ArgumentOutOfRangeException >(() => {
        let _ = RandomNumberGenerator.GetBytes(- 1);
    }
    );
}
