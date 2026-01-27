namespace Std.Security.Cryptography;
import Std.Span;
/// <summary>Base class for HMAC implementations.</summary>
public abstract class Hmac
{
    public abstract void SetKey(ReadOnlySpan <byte >key);
    public abstract void Append(ReadOnlySpan <byte >data);
    public abstract int FinalizeHash(Span <byte >destination);
    public abstract void Reset();
}
