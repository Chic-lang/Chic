namespace Std;
import Std.Core;
import Std.Span;
import Std.Testing;
testcase Given_string_hashcode_on_runtime_constructed_value_When_executed_Then_deterministic()
{
    var bytes = Span <byte >.StackAlloc(3);
    bytes[0] = (byte) 'a';
    bytes[1] = (byte) 'b';
    bytes[2] = (byte) 'c';
    let value = Utf8String.FromSpan(bytes.AsReadOnly());
    Assert.That(value).IsEqualTo("abc");
    let h1 = value.GetHashCode();
    let h2 = value.GetHashCode();
    Assert.That(h1 == h2).IsTrue();
    Assert.That(value == "abc").IsTrue();
    Assert.That(value != "abd").IsTrue();
}
