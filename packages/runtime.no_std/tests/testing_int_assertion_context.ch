namespace Std.Testing;
import Std;
/// <summary>Fluent assertions for 32-bit signed integers.</summary>
public struct IntAssertionContext
{
    private readonly int _actual;
    public init(int value) {
        _actual = value;
    }
    public IntAssertionContext IsEqualTo(int expected) {
        if (_actual != expected)
        {
            throw new AssertionFailedException(FormatExpectedActual(expected, _actual));
        }
        return this;
    }
    public IntAssertionContext IsNotEqualTo(int unexpected) {
        if (_actual == unexpected)
        {
            throw new AssertionFailedException(FormatExpectedActual(unexpected, _actual));
        }
        return this;
    }
    public static bool operator ! (IntAssertionContext context) => false;
    private static string FormatExpectedActual(int expected, int actual) {
        return "expected values to match but they differ";
    }
}