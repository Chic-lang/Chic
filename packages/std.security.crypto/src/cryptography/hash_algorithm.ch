namespace Std.Security.Cryptography;
import Std.Span;
import Std.Numeric;
/// <summary>Base class for incremental, span-first hash algorithms.</summary>
public abstract class HashAlgorithm
{
    public abstract int HashSizeBits {
        get;
    }
    public abstract void Append(ReadOnlySpan <byte >data);
    public abstract int FinalizeHash(Span <byte >destination);
    public virtual byte[] ComputeHash(ReadOnlySpan <byte >data) {
        Reset();
        Append(data);
        let sizeBytes = HashSizeBits / 8;
        var output = new byte[sizeBytes];
        let written = FinalizeHash(Span <byte >.FromArray(ref output));
        if (written <output.Length)
        {
            var trimmed = new byte[written];
            Span <byte >.FromArray(ref trimmed).Slice(0usize, NumericUnchecked.ToUSize(written)).CopyFrom(ReadOnlySpan <byte >.FromArray(ref output).Slice(0usize,
            NumericUnchecked.ToUSize(written)));
            output = trimmed;
        }
        Reset();
        return output;
    }
    public abstract void Reset();
}
