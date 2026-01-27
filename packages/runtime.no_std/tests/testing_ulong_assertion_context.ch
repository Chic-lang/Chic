namespace Std.Testing;
import Std;
/// <summary>Fluent assertions for 64-bit unsigned integers.</summary>
public struct ULongAssertionContext
{
    private readonly ulong _actual;
    public init(ulong value) {
        _actual = value;
    }
    public ULongAssertionContext IsEqualTo(ulong expected) {
        if (_actual != expected)
        {
            throw new AssertionFailedException(FormatExpectedActual(expected, _actual));
        }
        return this;
    }
    public ULongAssertionContext IsNotEqualTo(ulong unexpected) {
        if (_actual == unexpected)
        {
            throw new AssertionFailedException(FormatExpectedActual(unexpected, _actual));
        }
        return this;
    }
    public static bool operator !(ULongAssertionContext context) => false;
    private static string FormatExpectedActual(ulong expected, ulong actual) {
        return "expected values to match but they differ";
    }
}
