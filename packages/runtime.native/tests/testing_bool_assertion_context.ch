namespace Std.Runtime.Native.Testing;

public struct BoolAssertionContext
{
    private bool _value;

    public init(bool value) {
        _value = value;
    }

    public BoolAssertionContext IsTrue() {
        if (!_value)
        {
            Assert.Fail();
        }
        return this;
    }

    public BoolAssertionContext IsFalse() {
        if (_value)
        {
            Assert.Fail();
        }
        return this;
    }

    public BoolAssertionContext IsEqualTo(bool expected) {
        if (_value != expected)
        {
            Assert.Fail();
        }
        return this;
    }

    public BoolAssertionContext IsNotEqualTo(bool unexpected) {
        if (_value == unexpected)
        {
            Assert.Fail();
        }
        return this;
    }
}

