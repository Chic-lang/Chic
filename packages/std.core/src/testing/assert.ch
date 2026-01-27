namespace Std.Core.Testing;
public static class Assert
{
    public static BoolAssertionContext That(bool value) {
        return new BoolAssertionContext(value);
    }

    public static ValueAssertionContext <T >That <T >(T value) {
        return new ValueAssertionContext <T >(value);
    }
}
