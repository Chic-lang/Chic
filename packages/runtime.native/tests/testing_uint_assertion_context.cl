namespace Std.Runtime.Native.Testing;
public struct UIntAssertionContext
{
    private uint _value;
    public init(uint value) {
        _value = value;
    }
    public UIntAssertionContext IsEqualTo(uint expected) {
        if (_value != expected)
        {
            Assert.Fail();
        }
        return this;
    }
    public UIntAssertionContext IsNotEqualTo(uint unexpected) {
        if (_value == unexpected)
        {
            Assert.Fail();
        }
        return this;
    }
}
