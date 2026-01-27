namespace Std.Runtime.Native.Testing;

public struct USizeAssertionContext
{
    private usize _value;

    public init(usize value) {
        _value = value;
    }

    public USizeAssertionContext IsEqualTo(usize expected) {
        if (_value != expected)
        {
            Assert.Fail();
        }
        return this;
    }

    public USizeAssertionContext IsNotEqualTo(usize unexpected) {
        if (_value == unexpected)
        {
            Assert.Fail();
        }
        return this;
    }
}

