namespace Std.Runtime.Native.Testing;
public struct LongAssertionContext
{
    private long _value;
    public init(long value) {
        _value = value;
    }
    public LongAssertionContext IsEqualTo(long expected) {
        if (_value != expected)
        {
            Assert.Fail();
        }
        return this;
    }
    public LongAssertionContext IsNotEqualTo(long unexpected) {
        if (_value == unexpected)
        {
            Assert.Fail();
        }
        return this;
    }
}
