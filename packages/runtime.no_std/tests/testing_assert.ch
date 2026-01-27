namespace Std.Testing;
import Std;
/// <summary>
/// Entry point for fluent assertions in no-std runtime tests.
/// </summary>
public static class Assert
{
    public static BoolAssertionContext That(bool value) {
        return new BoolAssertionContext(value);
    }
    public static IntAssertionContext That(int value) {
        return new IntAssertionContext(value);
    }
    public static UIntAssertionContext That(uint value) {
        return new UIntAssertionContext(value);
    }
    public static LongAssertionContext That(long value) {
        return new LongAssertionContext(value);
    }
    public static ULongAssertionContext That(ulong value) {
        return new ULongAssertionContext(value);
    }
    public static USizeAssertionContext That(usize value) {
        return new USizeAssertionContext(value);
    }
    public static void Throws<TException>(ThrowingAction action)
    {
        if (action == null)
        {
            throw new AssertionFailedException("expected action to throw but received null delegate");
        }
        try {
            action();
        }
        catch(Exception ex) {
            if (ex is TException) {
                return;
            }
            throw new AssertionFailedException("expected exception of the requested type but caught a different exception");
        }
        throw new AssertionFailedException("expected exception of the requested type to be thrown");
    }
}
