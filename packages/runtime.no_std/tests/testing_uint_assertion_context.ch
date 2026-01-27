namespace Std.Testing;
import Std;
/// <summary>Fluent assertions for 32-bit unsigned integers.</summary>
public struct UIntAssertionContext
{
    private readonly uint _actual;
    public init(uint value) {
        _actual = value;
    }
    public UIntAssertionContext IsEqualTo(uint expected) {
        if (_actual != expected)
        {
            throw new AssertionFailedException(FormatExpectedActual(expected, _actual));
        }
        return this;
    }
    public UIntAssertionContext IsNotEqualTo(uint unexpected) {
        if (_actual == unexpected)
        {
            throw new AssertionFailedException(FormatExpectedActual(unexpected, _actual));
        }
        return this;
    }
    public static bool operator ! (UIntAssertionContext context) => false;
    private static string FormatExpectedActual(uint expected, uint actual) {
        return "expected values to match but they differ";
    }
}