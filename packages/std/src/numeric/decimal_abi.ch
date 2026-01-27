namespace Std.Numeric.Decimal;
import Std.Memory;
import Std.Runtime.Collections;
import Std.Span;
import Std.Numeric;
@Intrinsic @StructLayout(LayoutKind.Sequential) public struct DecimalRoundingEncoding
{
    public uint Value;
    public init(uint value) {
        Value = value;
    }
    public static DecimalRoundingEncoding FromMode(DecimalRoundingMode mode) {
        var encoded = 0u;
        if (mode == DecimalRoundingMode.TowardZero)
        {
            encoded = 1u;
        }
        else if (mode == DecimalRoundingMode.AwayFromZero)
        {
            encoded = 2u;
        }
        else if (mode == DecimalRoundingMode.TowardPositive)
        {
            encoded = 3u;
        }
        else if (mode == DecimalRoundingMode.TowardNegative)
        {
            encoded = 4u;
        }
        return new DecimalRoundingEncoding(encoded);
    }
}
@Intrinsic @StructLayout(LayoutKind.Sequential) public struct DecimalRuntimeCall
{
    public DecimalStatus Status;
    public decimal Value;
    public init(DecimalStatus status, decimal value) {
        Status = status;
        Value = value;
    }
}
@Intrinsic @StructLayout(LayoutKind.Sequential) public struct DecimalConstPtr
{
    public usize Pointer;
    public init(usize pointer) {
        Pointer = pointer;
    }
    public static DecimalConstPtr From(ReadOnlySpanPtr raw) {
        return new DecimalConstPtr(Pointer.AddressOfConst(raw.Data.Pointer));
    }
    public static DecimalConstPtr FromValue(ValueConstPtr handle) {
        return new DecimalConstPtr(Pointer.AddressOfConst(handle.Pointer));
    }
}
@Intrinsic @StructLayout(LayoutKind.Sequential) public struct DecimalMutPtr
{
    public usize Pointer;
    public init(usize pointer) {
        Pointer = pointer;
    }
    public static DecimalMutPtr From(SpanPtr raw) {
        return new DecimalMutPtr(Pointer.AddressOf(raw.Data.Pointer));
    }
    public static DecimalMutPtr FromValue(ValueMutPtr handle) {
        return new DecimalMutPtr(Pointer.AddressOf(handle.Pointer));
    }
}
public enum DecimalRoundingMode
{
    TiesToEven = 0, TowardZero = 1, AwayFromZero = 2, TowardPositive = 3, TowardNegative = 4,
}
public enum DecimalVectorizeHint
{
    None = 0, Decimal = 1,
}
public static class DecimalFlags
{
    public const uint Vectorize = 0x00000001u;
}
/// High-level helpers used by the decimal fast-paths to keep encoding and cloning
/// logic in Chic rather than the runtime.
public static class Intrinsics
{
    public static DecimalIntrinsicResult BuildResult(DecimalStatus status, decimal value, bool vectorized) {
        return new DecimalIntrinsicResult(status, value, DecimalIntrinsicVariant.Scalar);
    }
    public static DecimalRoundingEncoding EncodeRounding(DecimalRoundingMode mode) {
        return DecimalRoundingEncoding.FromMode(mode);
    }
    public static DecimalStatus CloneInto(DecimalConstPtr source, DecimalMutPtr destination) {
        unsafe {
            let srcPtr = (* const @readonly @expose_address byte) source.Pointer;
            let dstPtr = (* mut @expose_address byte) destination.Pointer;
            if (Std.Numeric.Pointer.IsNullConst (srcPtr) || Std.Numeric.Pointer.IsNull (dstPtr))
            {
                return DecimalStatus.InvalidPointer;
            }
            let src = Std.Runtime.Collections.ValuePointer.CreateConst(Std.Numeric.PointerIntrinsics.AsByteConst(srcPtr),
            __sizeof <decimal >(), __alignof <decimal >());
            let dst = Std.Runtime.Collections.ValuePointer.CreateMut(Std.Numeric.PointerIntrinsics.AsByteMut(dstPtr), __sizeof <decimal >(),
            __alignof <decimal >());
            Std.Memory.GlobalAllocator.Copy(dst, src, __sizeof <decimal >());
            return DecimalStatus.Success;
        }
    }
}
