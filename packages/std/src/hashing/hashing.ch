namespace Std.Hashing;
import Std.Span;
import Std.Runtime;
import Std.Runtime.Collections;
import Std.Numeric;
public interface IHasher
{
    void Reset(ref this);
    void Write(ref this, ReadOnlySpan <byte >data);
    ulong Finish(in this);
}
public struct DefaultHasher : IHasher, Copy
{
    private ulong _state;
    public init() {
        _state = OffsetBasis();
    }
    public void Reset(ref this) {
        _state = OffsetBasis();
    }
    public void Write(ref this, ReadOnlySpan <byte >data) {
        var idx = 0usize;
        while (idx <data.Length)
        {
            let b = data[idx];
            _state = (_state ^ NumericUnchecked.ToUInt64(b)) * Prime();
            idx += 1usize;
        }
    }
    public ulong Finish(in this) {
        return _state;
    }
    private static ulong OffsetBasis() => 14695981039346656037ul;
    private static ulong Prime() => 1099511628211ul;
}
public static class Hashing
{
    public static ulong HashValue <T, THasher >(in T value, THasher builder) where THasher : IHasher, Copy {
        var hasher = builder;
        hasher.Reset();
        let code = HashCodeOf <T >(in value);
        WriteU64 <THasher >(ref hasher, code);
        return hasher.Finish();
    }
    public static ulong HashCodeOf <T >(in T value) {
        let glue = __hash_glue_of <T >();
        if (glue == 0)
        {
            throw new Std.InvalidOperationException(Std.Runtime.StringRuntime.FromStr("type does not provide GetHashCode"));
        }
        unsafe {
            var * mut @expose_address T valuePtr = & value;
            let bytes = PointerIntrinsics.AsByteConstFromMut(valuePtr);
            let handle = ValuePointer.CreateConst(bytes, __sizeof <T >(), __alignof <T >());
            return Std.Runtime.HashRuntime.Invoke(glue, handle);
        }
    }
    public static void WriteU64 <THasher >(ref THasher hasher, ulong value) where THasher : IHasher {
        unsafe {
            var tmp = value;
            var * mut @expose_address ulong tmpPtr = & tmp;
            let bytes = PointerIntrinsics.AsByteConstFromMut(tmpPtr);
            let handle = ValuePointer.CreateConst(bytes, __sizeof <ulong >(), __alignof <ulong >());
            let span = ReadOnlySpan <byte >.FromValuePointer(handle, (usize) __sizeof <ulong >());
            hasher.Write(span);
        }
    }
}
