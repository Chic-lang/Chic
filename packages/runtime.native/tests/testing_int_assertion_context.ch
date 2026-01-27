namespace Std.Runtime.Native.Testing;

public struct IntAssertionContext
{
    private int _value;

    public init(int value) {
        _value = value;
    }

    public IntAssertionContext IsEqualTo(int expected) {
        if (_value != expected)
        {
            Assert.Fail();
        }
        return this;
    }

    public IntAssertionContext IsNotEqualTo(int unexpected) {
        if (_value == unexpected)
        {
            Assert.Fail();
        }
        return this;
    }
}

