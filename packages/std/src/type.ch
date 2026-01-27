namespace Std;
/// Minimal runtime type token backed by the compiler-provided type id intrinsic.
public readonly struct Type
{
    private readonly ulong _id;
    public init(ulong id) {
        _id = id;
    }
    public ulong Id => _id;
    public static Type Of <T >() => new Type(__type_id_of <T >());
    public override bool Equals(Object other) {
        return false;
    }
    public bool Equals(Type other) => _id == other._id;
    public override int GetHashCode() {
        unchecked {
            return(int) _id ^ (int)(_id >> 32);
        }
    }
    public static bool operator == (Type left, Type right) => left._id == right._id;
    public static bool operator != (Type left, Type right) => left._id != right._id;
}
