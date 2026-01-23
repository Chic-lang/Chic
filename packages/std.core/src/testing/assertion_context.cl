namespace Std.Core.Testing;
import Std.Core;
import Std.Numeric;
import Std.Runtime;
public struct ValueAssertionContext <T >
{
    private readonly T _value;
    public init(T value) {
        _value = value;
    }
    public ValueAssertionContext <T >IsEqualTo(T expected) {
        if (!AreEqual (expected))
        {
            throw new AssertionFailedException("assertion failed: values were not equal");
        }
        return this;
    }
    public ValueAssertionContext <T >IsNotEqualTo(T unexpected) {
        if (AreEqual (unexpected))
        {
            throw new AssertionFailedException("assertion failed: values were equal");
        }
        return this;
    }
    public ValueAssertionContext <T >IsNull() {
        var defaultValue = CoreIntrinsics.DefaultValue <T >();
        if (!AreEqual (defaultValue))
        {
            throw new AssertionFailedException("assertion failed: expected null but was non-null");
        }
        return this;
    }
    public ValueAssertionContext <T >IsNotNull() {
        var defaultValue = CoreIntrinsics.DefaultValue <T >();
        if (AreEqual (defaultValue))
        {
            throw new AssertionFailedException("assertion failed: expected non-null but was null");
        }
        return this;
    }
    public ValueAssertionContext <T >IsTrue() {
        if (__type_id_of <T > () != __type_id_of <bool > ())
        {
            throw new AssertionFailedException("assertion failed: expected a boolean value");
        }
        let defaultValue = CoreIntrinsics.DefaultValue <T >();
        if (AreEqual (defaultValue))
        {
            throw new AssertionFailedException("assertion failed: expected true but was false");
        }
        return this;
    }
    public ValueAssertionContext <T >IsFalse() {
        if (__type_id_of <T > () != __type_id_of <bool > ())
        {
            throw new AssertionFailedException("assertion failed: expected a boolean value");
        }
        let defaultValue = CoreIntrinsics.DefaultValue <T >();
        if (!AreEqual (defaultValue))
        {
            throw new AssertionFailedException("assertion failed: expected false but was true");
        }
        return this;
    }
    public static bool operator !(ValueAssertionContext <T >context) => false;
    private bool AreEqual(T other) {
        let eqFn = (isize) __eq_glue_of <T >();
        if (eqFn == 0isize)
        {
            throw new AssertionFailedException("assertion failed: expected values to support equality");
        }
        unsafe {
            var * mut @expose_address T leftPtr = & _value;
            var * mut @expose_address T rightPtr = & other;
            let leftBytes = PointerIntrinsics.AsByteConstFromMut(leftPtr);
            let rightBytes = PointerIntrinsics.AsByteConstFromMut(rightPtr);
            return EqRuntime.Invoke(eqFn, leftBytes, rightBytes);
        }
    }
}
