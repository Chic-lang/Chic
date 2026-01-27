namespace Std.Testing;
import Std;
/// <summary>Fluent assertions for pointer-sized unsigned integers.</summary>
public struct USizeAssertionContext
{
    private readonly usize _actual;
    public init(usize value) {
        _actual = value;
    }
    public USizeAssertionContext IsEqualTo(usize expected) {
        if (_actual != expected)
        {
            throw new AssertionFailedException(FormatExpectedActual(expected, _actual));
        }
        return this;
    }
    public USizeAssertionContext IsNotEqualTo(usize unexpected) {
        if (_actual == unexpected)
        {
            throw new AssertionFailedException(FormatExpectedActual(unexpected, _actual));
        }
        return this;
    }
    public static bool operator ! (USizeAssertionContext context) => false;
    private static string FormatExpectedActual(usize expected, usize actual) {
        return "expected values to match but they differ";
    }
}