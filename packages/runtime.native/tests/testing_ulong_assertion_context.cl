namespace Std.Runtime.Native.Testing;

public struct ULongAssertionContext
{
    private ulong _value;

    public init(ulong value) {
        _value = value;
    }

    public ULongAssertionContext IsEqualTo(ulong expected) {
        if (_value != expected)
        {
            Assert.Fail();
        }
        return this;
    }

    public ULongAssertionContext IsNotEqualTo(ulong unexpected) {
        if (_value == unexpected)
        {
            Assert.Fail();
        }
        return this;
    }
}

