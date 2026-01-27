namespace Std.Security.Cryptography;
import Std.Span;
/// <summary>Transform interface for symmetric cryptography operations.</summary>
public interface ICryptoTransform
{
    int InputBlockSize {
        get;
    }
    int OutputBlockSize {
        get;
    }
    bool CanTransformMultipleBlocks {
        get;
    }
    bool CanReuseTransform {
        get;
    }
    int TransformBlock(ReadOnlySpan <byte >input, Span <byte >output);
    int TransformFinalBlock(ReadOnlySpan <byte >input, Span <byte >output);
    void Reset();
}
