namespace Std.Testing;
internal struct EquatableType
{
    public int Value;
    internal static bool operator == (EquatableType left, EquatableType right) => left.Value == right.Value;
    @allow(dead_code) internal static bool operator != (EquatableType left, EquatableType right) => left.Value != right.Value;
}
