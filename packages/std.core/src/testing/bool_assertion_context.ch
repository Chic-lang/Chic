namespace Std.Core.Testing;
public struct BoolAssertionContext
{
    private readonly bool _actual;
    public init(bool value) {
        _actual = value;
    }
    public BoolAssertionContext IsTrue() {
        if (!_actual)
        {
            throw new AssertionFailedException("expected true but was false");
        }
        return this;
    }
    public BoolAssertionContext IsFalse() {
        if (_actual)
        {
            throw new AssertionFailedException("expected false but was true");
        }
        return this;
    }
    public BoolAssertionContext IsEqualTo(bool expected) {
        if (_actual != expected)
        {
            throw new AssertionFailedException(FormatExpectedActual(expected, _actual));
        }
        return this;
    }
    public BoolAssertionContext IsNotEqualTo(bool unexpected) {
        if (_actual == unexpected)
        {
            throw new AssertionFailedException("expected values to differ but they matched");
        }
        return this;
    }
    public static bool operator ! (BoolAssertionContext context) => false;
    private static string FormatExpectedActual(bool expected, bool actual) {
        return "expected " + (expected ? "true" : "false") + " but was " + (actual ? "true" : "false");
    }
}

