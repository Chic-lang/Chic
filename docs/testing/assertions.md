# Std.Testing Assertions

`Std.Testing` provides fluent assertions for inline `testcase` blocks.
Import the namespace and use `Assert.That(value)` to capture the actual value, then assert a single expectation per testcase:

- Equality: `.IsEqualTo(expected)`, `.IsNotEqualTo(expected)`
- Approximate numbers: `.IsCloseTo(target, tolerance)`
- Booleans: `.IsTrue()`, `.IsFalse()`
- Nullability: `.IsNull()`, `.IsNotNull()`
- Strings: `.Contains(substring)`, `.StartsWith(prefix)`, `.EndsWith(suffix)`
- Spans: `.HasLength(expected)`, `.IsEmpty()`, `.IsNotEmpty()`, `.IsEqualTo(expected)`
- Exceptions: `Assert.Throws<ExceptionType>(fn() -> void action)`

Failures throw `Std.Testing.AssertionFailedException` with actionable messages
including the actual and expected values. All helpers return the same
`AssertionContext`, but keep one assertion per testcase for clearer diagnostics.

Example usage:

```cl
import Std;
import Std.Testing;

testcase ValidatesMath()
{
    Assert.That(Math.Hypot(3, 4)).IsEqualTo(5);
}

testcase ValidatesText()
{
    Assert.That("chic").Contains("chic");
}

testcase ValidatesBoolean()
{
    Assert.That(false).IsFalse();
}

testcase ThrowsWhenInvalid()
{
    Assert.Throws<InvalidOperationException>(fn() -> void {
        throw new InvalidOperationException("boom");
    });
}
```
