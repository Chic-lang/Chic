namespace Std.Testing;
import Std;
/// <summary>Fluent assertions for 64-bit signed integers.</summary>
public struct LongAssertionContext
{
    private readonly long _actual;
    public init(long value) {
        _actual = value;
    }
    public LongAssertionContext IsEqualTo(long expected) {
        if (_actual != expected)
        {
            throw new AssertionFailedException(FormatExpectedActual(expected, _actual));
        }
        return this;
    }
    public LongAssertionContext IsNotEqualTo(long unexpected) {
        if (_actual == unexpected)
        {
            throw new AssertionFailedException(FormatExpectedActual(unexpected, _actual));
        }
        return this;
    }
    public static bool operator !(LongAssertionContext context) => false;
    private static string FormatExpectedActual(long expected, long actual) {
        return "expected values to match but they differ";
    }
}
