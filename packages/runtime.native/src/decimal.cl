namespace Std.Runtime.Native;
// Chic-native decimal runtime that mirrors the Rust ABI but executes all semantic
// work in Chic. This keeps Rust code as a thin shim when `chic_native_runtime`
// is enabled.
@repr(c) public struct Decimal128Parts
{
    public u32 lo;
    public u32 mid;
    public u32 hi;
    public u32 flags;
}
@repr(c) public struct DecimalConstPtr
{
    public * const Decimal128Parts Pointer;
}
@repr(c) public struct DecimalMutPtr
{
    public * mut Decimal128Parts Pointer;
}
@repr(c) public struct DecimalRoundingAbi
{
    public u32 value;
}
@repr(c) public struct DecimalRuntimeResult
{
    public i32 status;
    public Decimal128Parts value;
}
public static class DecimalFlags
{
    public const uint Vectorize = 0x00000001u;
}
public enum DecimalRuntimeStatus
{
    Success = 0, Overflow = 1, DivideByZero = 2, InvalidRounding = 3, InvalidFlags = 4, InvalidPointer = 5, InvalidOperand = 6,
}
public enum DecimalRoundingMode
{
    TiesToEven = 0, TowardZero = 1, AwayFromZero = 2, TowardPositive = 3, TowardNegative = 4,
}
internal struct DecValue
{
    public u64 Low;
    public u64 High;
    public u32 Scale;
    public bool Negative;
}
internal enum DecimalBinaryKind
{
    Add, Sub, Mul, Div, Rem,
}
internal enum DecimalTernaryKind
{
    Fma,
}
internal static class DecimalEnv
{
    private static DecimalRoundingMode _rounding;
    public static DecimalRoundingMode Rounding => _rounding;
    public static void SetRounding(DecimalRoundingMode mode) {
        _rounding = mode;
    }
}
private const usize DECIMAL_SIZE = sizeof(Decimal128Parts);
private const usize DECIMAL_ALIGN = sizeof(u32);
private const usize DECIMAL_RESULT_VALUE_OFFSET = 16usize;
private static Decimal128Parts Zero() {
    return new Decimal128Parts {
        lo = 0u32, mid = 0u32, hi = 0u32, flags = 0u32
    }
    ;
}
private unsafe static bool IsNullConstParts(* const @readonly Decimal128Parts ptr) {
    return NativePtr.ToIsize(ptr as * const @readonly byte) == 0isize;
}
private unsafe static bool IsNullMutParts(* mut Decimal128Parts ptr) {
    return NativePtr.ToIsize(ptr as * mut @expose_address byte) == 0isize;
}
private unsafe static bool IsNullResultPtr(* mut DecimalRuntimeResult ptr) {
    return NativePtr.ToIsize(ptr as * mut @expose_address byte) == 0isize;
}
private static Decimal128Parts EncodeDec(DecValue value) {
    var magnitude = Magnitude(value);
    let isNegative = value.Negative;
    let lo = (u32)(magnitude & 0xFFFF_FFFFu128);
    let mid = (u32)((magnitude >> 32) & 0xFFFF_FFFFu128);
    let hi = (u32)((magnitude >> 64) & 0xFFFF_FFFFu128);
    let flags = ((u32)(value.Scale & 0xFFu32) << 16) | (isNegative ?0x8000_0000u32 : 0u32);
    return new Decimal128Parts {
        lo = lo, mid = mid, hi = hi, flags = flags
    }
    ;
}
private static DecValue DecodeParts(Decimal128Parts parts) {
    let flags = parts.flags;
    let scale = (u32)((flags >> 16) & 0xFFu32);
    let sign = (flags & 0x8000_0000u32) != 0u32;
    let lo = parts.lo;
    let mid = parts.mid;
    let hi = parts.hi;
    let magnitude = ((u128) hi << 64) | ((u128) mid << 32) | (u128) lo;
    return Compose(magnitude, scale, sign);
}
private static DecimalRuntimeResult MakeResult(DecimalRuntimeStatus status, DecValue value) {
    var normalized = value;
    NormalizeScale(ref normalized);
    return new DecimalRuntimeResult {
        status = (i32) status, value = ToParts(normalized)
    }
    ;
}
private static DecimalRuntimeResult Failure(DecimalRuntimeStatus status) {
    return MakeResult(status, DecZero());
}
private static DecimalRuntimeResult Success(DecValue value) {
    return MakeResult(DecimalRuntimeStatus.Success, value);
}
private static DecimalRuntimeResult MakeSimpleBinaryResult(* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts rhs,
DecimalBinaryKind kind, bool checkDivideByZero) {
    var leftParts = Zero();
    var rightParts = Zero();
    unsafe {
        let lhs_is_null = IsNullConstParts(lhs);
        let rhs_is_null = IsNullConstParts(rhs);
        if (lhs_is_null || rhs_is_null)
        {
            return Failure(DecimalRuntimeStatus.InvalidPointer);
        }
        leftParts = * lhs;
        rightParts = * rhs;
    }
    var left = FromParts(leftParts);
    var right = FromParts(rightParts);
    let maxScale = 28u32;
    if (left.Scale > maxScale || right.Scale > maxScale)
    {
        return Failure(DecimalRuntimeStatus.InvalidOperand);
    }
    if (checkDivideByZero && Magnitude (right) == 0u128)
    {
        return Failure(DecimalRuntimeStatus.DivideByZero);
    }
    var value = DecZero();
    var ok = true;
    if (kind == DecimalBinaryKind.Add)
    {
        ok = TryAdd(left, right, out value);
    }
    else if (kind == DecimalBinaryKind.Sub)
    {
        ok = TrySub(left, right, out value);
    }
    else if (kind == DecimalBinaryKind.Mul)
    {
        ok = TryMul(left, right, out value);
    }
    else if (kind == DecimalBinaryKind.Div)
    {
        ok = TryDiv(left, right, out value);
    }
    else
    {
        ok = TryRem(left, right, out value);
    }
    if (! ok)
    {
        value = DecZero();
        return Failure(DecimalRuntimeStatus.InvalidOperand);
    }
    return Success(value);
}
private static DecValue DecZero() {
    return new DecValue {
        Low = 0u64, High = 0u64, Scale = 0u32, Negative = false
    }
    ;
}
private static u128 Magnitude(DecValue value) {
    return((u128) value.High << 64) | (u128) value.Low;
}
private static DecValue Compose(u128 magnitude, u32 scale, bool negative) {
    return new DecValue {
        Low = (u64)(magnitude & 0xFFFF_FFFF_FFFF_FFFFu128), High = (u64)(magnitude >> 64), Scale = scale, Negative = negative,
    }
    ;
}
private static DecValue FromSigned(i128 coeff, u32 scale) {
    let isNegative = coeff <0;
    var magnitude = isNegative ?(u128)(- coeff) : (u128) coeff;
    return Compose(magnitude, scale, isNegative);
}
private static i128 SignedCoeff(DecValue value) {
    var coeff = (i128) Magnitude(value);
    return value.Negative ?- coeff : coeff;
}
private static bool TryPow10(u32 exp, out i128 value) {
    if (exp > 28u32)
    {
        value = 0i128;
        return false;
    }
    var result = SignedCoeff(FromSigned(1i128, 0u32));
    var index = 0u32;
    while (index <exp)
    {
        result = result * 10i128;
        index = index + 1u32;
    }
    value = result;
    return true;
}
private static void NormalizeScale(ref DecValue value) {
    var magnitude = Magnitude(value);
    while (value.Scale >0u32 && (magnitude % 10u128) == 0u128)
    {
        magnitude = magnitude / 10u128;
        value.Scale = value.Scale - 1u32;
    }
    value.Low = (u64)(magnitude & 0xFFFF_FFFF_FFFF_FFFFu128);
    value.High = (u64)(magnitude >> 64);
}
private static i128 RoundQuotient(i128 numerator, i128 denominator, DecimalRoundingMode mode) {
    if (denominator == 0i128)
    {
        return 0i128;
    }
    let negative = (numerator <0i128) ^ (denominator <0i128);
    var abs_num = numerator;
    if (abs_num <0i128)
    {
        abs_num = - abs_num;
    }
    var abs_den = denominator;
    if (abs_den <0i128)
    {
        abs_den = - abs_den;
    }
    var abs_q = abs_num / abs_den;
    let remainder = abs_num % abs_den;
    if (remainder == 0i128)
    {
        return negative ?- abs_q : abs_q;
    }
    var increment = false;
    if (mode == DecimalRoundingMode.TiesToEven)
    {
        var twice = remainder * 2i128;
        if (twice >abs_den)
        {
            increment = true;
        }
        else if (twice == abs_den && (abs_q % 2i128) != 0i128)
        {
            increment = true;
        }
    }
    else if (mode == DecimalRoundingMode.TowardZero)
    {
        increment = false;
    }
    else if (mode == DecimalRoundingMode.AwayFromZero)
    {
        increment = true;
    }
    else if (mode == DecimalRoundingMode.TowardPositive)
    {
        increment = ! negative;
    }
    else if (mode == DecimalRoundingMode.TowardNegative)
    {
        increment = negative;
    }
    if (increment)
    {
        abs_q = abs_q + 1i128;
    }
    return negative ?- abs_q : abs_q;
}
private static bool ScaleUp(ref DecValue value, u32 delta) {
    if (delta == 0u32)
    {
        return true;
    }
    var factor = SignedCoeff(DecZero());
    if (! TryPow10 (delta, out factor)) {
        return false;
    }
    var magnitude = Magnitude(value) * (u128) factor;
    value.Low = (u64)(magnitude & 0xFFFF_FFFF_FFFF_FFFFu128);
    value.High = (u64)(magnitude >> 64);
    value.Scale = value.Scale + delta;
    return true;
}
	private static bool AlignScales(DecValue lhs, DecValue rhs, out DecValue lhsOut, out DecValue rhsOut, out u32 scale) {
	    lhsOut = lhs;
	    rhsOut = rhs;
	    if (lhsOut.Scale == rhsOut.Scale)
	    {
	        scale = lhsOut.Scale;
	        return true;
	    }
	    if (lhsOut.Scale <rhsOut.Scale)
	    {
	        let delta = rhsOut.Scale - lhsOut.Scale;
	        if (! ScaleUp (ref lhsOut, delta)) {
	            scale = 0u32;
	            return false;
	        }
	        scale = lhsOut.Scale;
	        return true;
	    }
	    let deltaRight = lhsOut.Scale - rhsOut.Scale;
	    if (! ScaleUp (ref rhsOut, deltaRight)) {
	        scale = 0u32;
	        return false;
	    }
	    scale = rhsOut.Scale;
	    return true;
	}
private static bool TryAdd(DecValue lhs, DecValue rhs, out DecValue result) {
    var left = lhs;
    var right = rhs;
    var scale = 0u32;
    if (! AlignScales (left, right, out left, out right, out scale)) {
        result = DecZero();
        return false;
    }
    var sum = SignedCoeff(left) + SignedCoeff(right);
    result = FromSigned(sum, scale);
    NormalizeScale(ref result);
    return true;
}
private static bool TrySub(DecValue lhs, DecValue rhs, out DecValue result) {
    var left = lhs;
    var right = rhs;
    var scale = 0u32;
    if (! AlignScales (left, right, out left, out right, out scale)) {
        result = DecZero();
        return false;
    }
    var diff = SignedCoeff(left) - SignedCoeff(right);
    result = FromSigned(diff, scale);
    NormalizeScale(ref result);
    return true;
}
private static bool TryMul(DecValue lhs, DecValue rhs, out DecValue result) {
    var product = SignedCoeff(lhs) * SignedCoeff(rhs);
    var value = FromSigned(product, lhs.Scale + rhs.Scale);
    if (value.Scale > 28u32)
    {
        let delta = value.Scale - 28u32;
        var divisor = SignedCoeff(DecZero());
        if (! TryPow10 (delta, out divisor)) {
            result = DecZero();
            return false;
        }
        let rounded = RoundQuotient(SignedCoeff(value), divisor, ActiveRounding());
        value = FromSigned(rounded, 28u32);
    }
    NormalizeScale(ref value);
    result = value;
    return true;
}
private static bool TryDiv(DecValue lhs, DecValue rhs, out DecValue result) {
    if (Magnitude (rhs) == 0u128)
    {
        result = DecZero();
        return false;
    }
    var targetScale = lhs.Scale;
    if (targetScale <rhs.Scale)
    {
        targetScale = rhs.Scale;
    }
    if (targetScale <6u32)
    {
        targetScale = 6u32;
    }
    if (targetScale > 28u32)
    {
        targetScale = 28u32;
    }
    let adjust = targetScale + rhs.Scale - lhs.Scale;
    var factor = SignedCoeff(DecZero());
    if (! TryPow10 (adjust, out factor)) {
        result = DecZero();
        return false;
    }
    var scaledNumerator = SignedCoeff(lhs) * factor;
    var divisor = SignedCoeff(rhs);
    var value = FromSigned(RoundQuotient(scaledNumerator, divisor, ActiveRounding()), targetScale);
    NormalizeScale(ref value);
    result = value;
    return true;
}
private static bool TryRem(DecValue lhs, DecValue rhs, out DecValue result) {
    if (Magnitude (rhs) == 0u128)
    {
        result = DecZero();
        return false;
    }
    var left = lhs;
    var right = rhs;
    var scale = 0u32;
    if (! AlignScales (left, right, out left, out right, out scale)) {
        result = DecZero();
        return false;
    }
    var value = FromSigned(SignedCoeff(left) % SignedCoeff(right), scale);
    NormalizeScale(ref value);
    result = value;
    return true;
}
private static bool TryFma(DecValue lhs, DecValue multiplicand, DecValue addend, out DecValue result) {
    var finalResult = DecZero();
    var product = DecZero();
    if (! TryMul (lhs, multiplicand, out product)) {
        result = DecZero();
        return false;
    }
    var addendLocal = addend;
    if (! TryAdd (product, addendLocal, out finalResult)) {
        result = DecZero();
        return false;
    }
    result = finalResult;
    return true;
}
private static DecimalRuntimeResult SuccessZero() {
    return Success(DecZero());
}
private static bool ValidateFlags(uint flags, bool requireVectorize) {
    let masked = flags & ~ DecimalFlags.Vectorize;
    if (masked != 0u)
    {
        return false;
    }
    let vectorize = (flags & DecimalFlags.Vectorize) != 0u;
    if (requireVectorize && ! vectorize)
    {
        return false;
    }
    if (! requireVectorize && vectorize)
    {
        return false;
    }
    return true;
}
private static bool TryDecodeRounding(DecimalRoundingAbi abi, out DecimalRoundingMode mode) {
    if (abi.value == 0u)
    {
        mode = DecimalRoundingMode.TiesToEven;
        return true;
    }
    if (abi.value == 1u)
    {
        mode = DecimalRoundingMode.TowardZero;
        return true;
    }
    if (abi.value == 2u)
    {
        mode = DecimalRoundingMode.AwayFromZero;
        return true;
    }
    if (abi.value == 3u)
    {
        mode = DecimalRoundingMode.TowardPositive;
        return true;
    }
    if (abi.value == 4u)
    {
        mode = DecimalRoundingMode.TowardNegative;
        return true;
    }
    mode = DecimalRoundingMode.TiesToEven;
    return false;
}
private unsafe static ValueConstPtr PartsConst(* const @readonly Decimal128Parts ptr) {
    return new ValueConstPtr {
        Pointer = NativePtr.AsConstPtr(ptr), Size = DECIMAL_SIZE, Alignment = DECIMAL_ALIGN,
    }
    ;
}
private unsafe static ValueMutPtr PartsMut(* mut Decimal128Parts ptr) {
    return new ValueMutPtr {
        Pointer = NativePtr.AsMutPtr(ptr), Size = DECIMAL_SIZE, Alignment = DECIMAL_ALIGN,
    }
    ;
}
private unsafe static ValueConstPtr LocalPartsConst(ref Decimal128Parts value) {
    var * const @readonly byte raw = & value;
    return new ValueConstPtr {
        Pointer = NativePtr.AsByteConst(raw), Size = DECIMAL_SIZE, Alignment = DECIMAL_ALIGN,
    }
    ;
}
private unsafe static ValueConstPtr LocalI32Const(ref i32 value) {
    var * const @readonly byte raw = & value;
    return new ValueConstPtr {
        Pointer = NativePtr.AsByteConst(raw), Size = sizeof(i32), Alignment = sizeof(i32),
    }
    ;
}
private unsafe static ValueMutPtr PartsMutFromPtr(* mut Decimal128Parts ptr) {
    var * mut byte raw = ptr;
    return new ValueMutPtr {
        Pointer = NativePtr.AsByteMut(raw), Size = DECIMAL_SIZE, Alignment = DECIMAL_ALIGN,
    }
    ;
}
private static Decimal128Parts ToParts(DecValue value) {
    return EncodeDec(value);
}
private static DecValue FromParts(Decimal128Parts parts) {
    return DecodeParts(parts);
}
private static bool TryLoadDecimal(* const @readonly Decimal128Parts ptr, out DecValue value) {
    unsafe {
        if (IsNullConstParts (ptr))
        {
            value = DecZero();
            return false;
        }
        // Directly dereference the incoming pointer to avoid any ABI drift in
        // helper copy routines.
        let parts = * ptr;
        value = FromParts(parts);
    }
    return true;
}
private static void WriteResult(* mut DecimalRuntimeResult outPtr, DecimalRuntimeResult value) {
    unsafe {
        if (IsNullResultPtr (outPtr))
        {
            return;
        }
        * outPtr = value;
    }
}
private static DecimalRuntimeResult HandleRounding(DecimalRoundingMode mode) {
    DecimalEnv.SetRounding(mode);
    return SuccessZero();
}
private static DecimalRoundingMode ActiveRounding() {
    return DecimalEnv.Rounding;
}
private static DecimalRuntimeStatus BinaryOpValue(* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts rhs,
DecimalRoundingAbi rounding, uint flags, bool requireVectorize, bool checkDivideByZero, DecimalBinaryKind kind, out DecValue resultValue) {
    var resultLocal = DecZero();
    if (! ValidateFlags (flags, requireVectorize))
    {
        resultValue = resultLocal;
        return DecimalRuntimeStatus.InvalidFlags;
    }
    var mode = DecimalRoundingMode.TiesToEven;
    if (! TryDecodeRounding (rounding, out mode)) {
        resultValue = resultLocal;
        return DecimalRuntimeStatus.InvalidRounding;
    }
    let roundCheck = HandleRounding(mode);
    if (roundCheck.status != (int) DecimalRuntimeStatus.Success)
    {
        resultValue = resultLocal;
        return(DecimalRuntimeStatus) roundCheck.status;
    }
    var lhsValue = DecZero();
    var rhsValue = DecZero();
    if (! TryLoadDecimal (lhs, out lhsValue) || ! TryLoadDecimal(rhs, out rhsValue)) {
        resultValue = resultLocal;
        return DecimalRuntimeStatus.InvalidPointer;
    }
    if ( (checkDivideByZero || kind == DecimalBinaryKind.Rem) && Magnitude (rhsValue) == 0u128)
    {
        resultValue = resultLocal;
        return DecimalRuntimeStatus.DivideByZero;
    }
    var ok = true;
    if (kind == DecimalBinaryKind.Add)
    {
        ok = TryAdd(lhsValue, rhsValue, out resultLocal);
    }
    else if (kind == DecimalBinaryKind.Sub)
    {
        ok = TrySub(lhsValue, rhsValue, out resultLocal);
    }
    else if (kind == DecimalBinaryKind.Mul)
    {
        ok = TryMul(lhsValue, rhsValue, out resultLocal);
    }
    else if (kind == DecimalBinaryKind.Div)
    {
        ok = TryDiv(lhsValue, rhsValue, out resultLocal);
    }
    else
    {
        ok = TryRem(lhsValue, rhsValue, out resultLocal);
    }
    if (! ok)
    {
        resultValue = resultLocal;
        return DecimalRuntimeStatus.InvalidOperand;
    }
    resultValue = resultLocal;
    return DecimalRuntimeStatus.Success;
}
private static DecimalRuntimeResult BinaryOp(* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts rhs,
DecimalRoundingAbi rounding, uint flags, bool requireVectorize, bool checkDivideByZero, DecimalBinaryKind kind) {
    var value = DecZero();
    let status = BinaryOpValue(lhs, rhs, rounding, flags, requireVectorize, checkDivideByZero, kind, out value);
    return MakeResult(status, value);
}
private static DecimalRuntimeResult TernaryOp(* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts multiplicand,
* const @readonly Decimal128Parts addend, DecimalRoundingAbi rounding, uint flags, bool requireVectorize, DecimalTernaryKind kind) {
    if (! ValidateFlags (flags, requireVectorize))
    {
        return Failure(DecimalRuntimeStatus.InvalidFlags);
    }
    var mode = DecimalRoundingMode.TiesToEven;
    if (! TryDecodeRounding (rounding, out mode)) {
        return Failure(DecimalRuntimeStatus.InvalidRounding);
    }
    let roundCheck = HandleRounding(mode);
    if (roundCheck.status != (int) DecimalRuntimeStatus.Success)
    {
        return roundCheck;
    }
    var lhsValue = DecZero();
    var mulValue = DecZero();
    var addendValue = DecZero();
    if (! TryLoadDecimal (lhs, out lhsValue) || ! TryLoadDecimal(multiplicand, out mulValue) || ! TryLoadDecimal(addend,
    out addendValue)) {
        return Failure(DecimalRuntimeStatus.InvalidPointer);
    }
    if (kind != DecimalTernaryKind.Fma)
    {
        return Failure(DecimalRuntimeStatus.InvalidOperand);
    }
    var resultValue = DecZero();
    if (! TryFma (lhsValue, mulValue, addendValue, out resultValue)) {
        return Failure(DecimalRuntimeStatus.InvalidOperand);
    }
    return Success(resultValue);
}
private static DecimalRuntimeResult SumCore(DecimalConstPtr values, usize len, DecimalRoundingAbi rounding, uint flags, bool requireVectorize) {
    var mode = DecimalRoundingMode.TiesToEven;
    if (! ValidateFlags (flags, requireVectorize))
    {
        return Failure(DecimalRuntimeStatus.InvalidFlags);
    }
    if (! TryDecodeRounding (rounding, out mode)) {
        return Failure(DecimalRuntimeStatus.InvalidRounding);
    }
    let roundCheck = HandleRounding(mode);
    if (roundCheck.status != (int) DecimalRuntimeStatus.Success)
    {
        return roundCheck;
    }
    if (len == 0)
    {
        return SuccessZero();
    }
    unsafe {
        if (IsNullConstParts (values.Pointer))
        {
            return Failure(DecimalRuntimeStatus.InvalidPointer);
        }
    }
    var total = DecZero();
    let maxScale = 28u32;
    unsafe {
        let basePtr = NativePtr.AsConstPtr(values.Pointer);
        var index = 0usize;
        while (index <len)
        {
            let offset = (isize)(index * DECIMAL_SIZE);
            var parts = Zero();
            var src = new ValueConstPtr {
                Pointer = NativePtr.OffsetConst(basePtr, offset), Size = DECIMAL_SIZE, Alignment = DECIMAL_ALIGN,
            }
            ;
            NativeAlloc.Copy(PartsMutFromPtr(& parts), src, DECIMAL_SIZE);
            var decoded = FromParts(parts);
            if (decoded.Scale > maxScale)
            {
                return Failure(DecimalRuntimeStatus.InvalidOperand);
            }
            if (! TryAdd (total, decoded, out total)) {
                return Failure(DecimalRuntimeStatus.InvalidOperand);
            }
            index = index + 1;
        }
    }
    return Success(total);
}
private static DecimalRuntimeResult DotCore(DecimalConstPtr lhs, DecimalConstPtr rhs, usize len, DecimalRoundingAbi rounding,
uint flags, bool requireVectorize) {
    var mode = DecimalRoundingMode.TiesToEven;
    if (! ValidateFlags (flags, requireVectorize))
    {
        return Failure(DecimalRuntimeStatus.InvalidFlags);
    }
    if (! TryDecodeRounding (rounding, out mode)) {
        return Failure(DecimalRuntimeStatus.InvalidRounding);
    }
    let roundCheck = HandleRounding(mode);
    if (roundCheck.status != (int) DecimalRuntimeStatus.Success)
    {
        return roundCheck;
    }
    if (len == 0)
    {
        return SuccessZero();
    }
    unsafe {
        let lhs_null = IsNullConstParts(lhs.Pointer);
        let rhs_null = IsNullConstParts(rhs.Pointer);
        if (lhs_null || rhs_null)
        {
            return Failure(DecimalRuntimeStatus.InvalidPointer);
        }
    }
    var total = DecZero();
    let maxScale = 28u32;
    unsafe {
        let lhsBase = NativePtr.AsConstPtr(lhs.Pointer);
        let rhsBase = NativePtr.AsConstPtr(rhs.Pointer);
        var index = 0usize;
        while (index <len)
        {
            let lhsOffset = (isize)(index * DECIMAL_SIZE);
            let rhsOffset = (isize)(index * DECIMAL_SIZE);
            var lhsParts = Zero();
            var rhsParts = Zero();
            NativeAlloc.Copy(PartsMutFromPtr(& lhsParts), new ValueConstPtr {
                Pointer = NativePtr.OffsetConst(lhsBase, lhsOffset), Size = DECIMAL_SIZE, Alignment = DECIMAL_ALIGN,
            }
            , DECIMAL_SIZE);
            NativeAlloc.Copy(PartsMutFromPtr(& rhsParts), new ValueConstPtr {
                Pointer = NativePtr.OffsetConst(rhsBase, rhsOffset), Size = DECIMAL_SIZE, Alignment = DECIMAL_ALIGN,
            }
            , DECIMAL_SIZE);
            var lhsVal = FromParts(lhsParts);
            var rhsVal = FromParts(rhsParts);
            if (lhsVal.Scale > maxScale || rhsVal.Scale > maxScale)
            {
                return Failure(DecimalRuntimeStatus.InvalidOperand);
            }
            var product = DecZero();
            if (! TryMul (lhsVal, rhsVal, out product)) {
                return Failure(DecimalRuntimeStatus.InvalidOperand);
            }
            if (! TryAdd (total, product, out total)) {
                return Failure(DecimalRuntimeStatus.InvalidOperand);
            }
            index = index + 1;
        }
    }
    return Success(total);
}
private static bool TryMulUsize(usize lhs, usize rhs, out usize result) {
    if (lhs == 0 || rhs == 0)
    {
        result = 0;
        return true;
    }
    let product = lhs * rhs;
    if (product / lhs != rhs)
    {
        result = 0;
        return false;
    }
    result = product;
    return true;
}
private static int MatMulCore(DecimalConstPtr left, usize leftRows, usize leftCols, DecimalConstPtr right, usize rightCols,
DecimalMutPtr dest, DecimalRoundingAbi rounding, uint flags, bool requireVectorize) {
    var status = DecimalRuntimeStatus.Success;
    if (! ValidateFlags (flags, requireVectorize))
    {
        status = DecimalRuntimeStatus.InvalidFlags;
    }
    var mode = DecimalRoundingMode.TiesToEven;
    if (status == DecimalRuntimeStatus.Success && ! TryDecodeRounding (rounding, out mode)) {
        status = DecimalRuntimeStatus.InvalidRounding;
    }
    if (status == DecimalRuntimeStatus.Success)
    {
        let roundCheck = HandleRounding(mode);
        if (roundCheck.status != (int) DecimalRuntimeStatus.Success)
        {
            return roundCheck.status;
        }
    }
    if (leftRows == 0 || rightCols == 0)
    {
        return(int) status;
    }
    unsafe {
        let left_null = IsNullConstParts(left.Pointer);
        let right_null = IsNullConstParts(right.Pointer);
        let dest_null = IsNullMutParts(dest.Pointer);
        if (left_null || right_null || dest_null)
        {
            return(int) DecimalRuntimeStatus.InvalidPointer;
        }
    }
    var expectedLeft = 0usize;
    var expectedRight = 0usize;
    var expectedDest = 0usize;
    if (! TryMulUsize (leftRows, leftCols, out expectedLeft) || ! TryMulUsize(leftCols, rightCols, out expectedRight) || ! TryMulUsize(leftRows,
    rightCols, out expectedDest)) {
        return(int) DecimalRuntimeStatus.InvalidOperand;
    }
    unsafe {
        let leftBase = NativePtr.AsConstPtr(left.Pointer);
        let rightBase = NativePtr.AsConstPtr(right.Pointer);
        let destBase = NativePtr.AsMutPtr(dest.Pointer);
        let maxScale = 28u32;
        var row = 0usize;
        while (row <leftRows)
        {
            var col = 0usize;
            while (col <rightCols)
            {
                var acc = DecZero();
                var k = 0usize;
                while (k <leftCols)
                {
                    let lhsIndex = row * leftCols + k;
                    let rhsIndex = k * rightCols + col;
                    var lhsParts = Zero();
                    var rhsParts = Zero();
                    NativeAlloc.Copy(PartsMutFromPtr(& lhsParts), new ValueConstPtr {
                        Pointer = NativePtr.OffsetConst(leftBase, (isize)(lhsIndex * DECIMAL_SIZE)), Size = DECIMAL_SIZE,
                        Alignment = DECIMAL_ALIGN,
                    }
                    , DECIMAL_SIZE);
                    NativeAlloc.Copy(PartsMutFromPtr(& rhsParts), new ValueConstPtr {
                        Pointer = NativePtr.OffsetConst(rightBase, (isize)(rhsIndex * DECIMAL_SIZE)), Size = DECIMAL_SIZE,
                        Alignment = DECIMAL_ALIGN,
                    }
                    , DECIMAL_SIZE);
                    var lhsVal = FromParts(lhsParts);
                    var rhsVal = FromParts(rhsParts);
                    if (lhsVal.Scale > maxScale || rhsVal.Scale > maxScale)
                    {
                        return(int) DecimalRuntimeStatus.InvalidOperand;
                    }
                    var product = DecZero();
                    if (! TryMul (lhsVal, rhsVal, out product)) {
                        return(int) DecimalRuntimeStatus.InvalidOperand;
                    }
                    if (! TryAdd (acc, product, out acc)) {
                        return(int) DecimalRuntimeStatus.InvalidOperand;
                    }
                    k = k + 1;
                }
                let destIndex = row * rightCols + col;
                var destSlot = ToParts(acc);
                NativeAlloc.Copy(new ValueMutPtr {
                    Pointer = NativePtr.OffsetMut(destBase, (isize)(destIndex * DECIMAL_SIZE)), Size = DECIMAL_SIZE, Alignment = DECIMAL_ALIGN,
                }
                , LocalPartsConst(ref destSlot), DECIMAL_SIZE);
                col = col + 1;
            }
            row = row + 1;
        }
    }
    let _ = expectedLeft;
    let _ = expectedRight;
    let _ = expectedDest;
    return(int) status;
}
@export("chic_rt_decimal_add") public static DecimalRuntimeResult chic_rt_decimal_add(* const @readonly Decimal128Parts lhs,
* const @readonly Decimal128Parts rhs, DecimalRoundingAbi rounding, uint flags) {
    return DecimalAddValue(lhs, rhs, rounding, flags);
}
private static DecimalRuntimeResult DecimalAddValue(* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts rhs,
DecimalRoundingAbi rounding, uint flags) {
    let _ = rounding;
    let _ = flags;
    return MakeSimpleBinaryResult(lhs, rhs, DecimalBinaryKind.Add, false);
}
@export("chic_rt_decimal_add_out") public static void chic_rt_decimal_add_out(* mut DecimalRuntimeResult outPtr,
* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts rhs, DecimalRoundingAbi rounding, uint flags) {
    let _ = rounding;
    let _ = flags;
    WriteResult(outPtr, DecimalAddValue(lhs, rhs, rounding, flags));
}
private static DecimalRuntimeResult DecimalSubValue(* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts rhs,
DecimalRoundingAbi rounding, uint flags) {
    let _ = rounding;
    let _ = flags;
    return MakeSimpleBinaryResult(lhs, rhs, DecimalBinaryKind.Sub, false);
}
@export("chic_rt_decimal_sub") public static DecimalRuntimeResult chic_rt_decimal_sub(* const @readonly Decimal128Parts lhs,
* const @readonly Decimal128Parts rhs, DecimalRoundingAbi rounding, uint flags) {
    return DecimalSubValue(lhs, rhs, rounding, flags);
}
@export("chic_rt_decimal_sub_out") public static void chic_rt_decimal_sub_out(* mut DecimalRuntimeResult outPtr,
* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts rhs, DecimalRoundingAbi rounding, uint flags) {
    let _ = rounding;
    let _ = flags;
    WriteResult(outPtr, DecimalSubValue(lhs, rhs, rounding, flags));
}
private static DecimalRuntimeResult DecimalMulValue(* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts rhs,
DecimalRoundingAbi rounding, uint flags) {
    let _ = rounding;
    let _ = flags;
    return MakeSimpleBinaryResult(lhs, rhs, DecimalBinaryKind.Mul, false);
}
@export("chic_rt_decimal_mul") public static DecimalRuntimeResult chic_rt_decimal_mul(* const @readonly Decimal128Parts lhs,
* const @readonly Decimal128Parts rhs, DecimalRoundingAbi rounding, uint flags) {
    return DecimalMulValue(lhs, rhs, rounding, flags);
}
@export("chic_rt_decimal_mul_out") public static void chic_rt_decimal_mul_out(* mut DecimalRuntimeResult outPtr,
* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts rhs, DecimalRoundingAbi rounding, uint flags) {
    let _ = rounding;
    let _ = flags;
    WriteResult(outPtr, DecimalMulValue(lhs, rhs, rounding, flags));
}
private static DecimalRuntimeResult DecimalDivValue(* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts rhs,
DecimalRoundingAbi rounding, uint flags) {
    let _ = rounding;
    let _ = flags;
    return MakeSimpleBinaryResult(lhs, rhs, DecimalBinaryKind.Div, true);
}
@export("chic_rt_decimal_div") public static DecimalRuntimeResult chic_rt_decimal_div(* const @readonly Decimal128Parts lhs,
* const @readonly Decimal128Parts rhs, DecimalRoundingAbi rounding, uint flags) {
    return DecimalDivValue(lhs, rhs, rounding, flags);
}
@export("chic_rt_decimal_div_out") public static void chic_rt_decimal_div_out(* mut DecimalRuntimeResult outPtr,
* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts rhs, DecimalRoundingAbi rounding, uint flags) {
    let _ = rounding;
    let _ = flags;
    WriteResult(outPtr, DecimalDivValue(lhs, rhs, rounding, flags));
}
private static DecimalRuntimeResult DecimalRemValue(* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts rhs,
DecimalRoundingAbi rounding, uint flags) {
    let _ = rounding;
    let _ = flags;
    return MakeSimpleBinaryResult(lhs, rhs, DecimalBinaryKind.Rem, true);
}
@export("chic_rt_decimal_rem") public static DecimalRuntimeResult chic_rt_decimal_rem(* const @readonly Decimal128Parts lhs,
* const @readonly Decimal128Parts rhs, DecimalRoundingAbi rounding, uint flags) {
    return DecimalRemValue(lhs, rhs, rounding, flags);
}
@export("chic_rt_decimal_rem_out") public static void chic_rt_decimal_rem_out(* mut DecimalRuntimeResult outPtr,
* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts rhs, DecimalRoundingAbi rounding, uint flags) {
    let _ = rounding;
    let _ = flags;
    WriteResult(outPtr, DecimalRemValue(lhs, rhs, rounding, flags));
}
private static DecimalRuntimeResult DecimalFmaValue(* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts multiplicand,
* const @readonly Decimal128Parts addend, DecimalRoundingAbi rounding, uint flags) {
    return TernaryOp(lhs, multiplicand, addend, rounding, flags, false, DecimalTernaryKind.Fma);
}
@export("chic_rt_decimal_fma") public static DecimalRuntimeResult chic_rt_decimal_fma(* const @readonly Decimal128Parts lhs,
* const @readonly Decimal128Parts multiplicand, * const @readonly Decimal128Parts addend, DecimalRoundingAbi rounding, uint flags) {
    return DecimalFmaValue(lhs, multiplicand, addend, rounding, flags);
}
@export("chic_rt_decimal_fma_out") public static void chic_rt_decimal_fma_out(* mut DecimalRuntimeResult outPtr,
* const @readonly Decimal128Parts lhs, * const @readonly Decimal128Parts multiplicand, * const @readonly Decimal128Parts addend,
DecimalRoundingAbi rounding, uint flags) {
    WriteResult(outPtr, DecimalFmaValue(lhs, multiplicand, addend, rounding, flags));
}
@export("chic_rt_decimal_clone") public static int chic_rt_decimal_clone(DecimalConstPtr source, DecimalMutPtr destination) {
    unsafe {
        let src_null = IsNullConstParts(source.Pointer);
        let dest_null = IsNullMutParts(destination.Pointer);
        if (src_null || dest_null)
        {
            return(int) DecimalRuntimeStatus.InvalidPointer;
        }
        var srcHandle = PartsConst(source.Pointer);
        var dstHandle = PartsMut(destination.Pointer);
        NativeAlloc.Copy(dstHandle, srcHandle, DECIMAL_SIZE);
    }
    return(int) DecimalRuntimeStatus.Success;
}
private static DecimalRuntimeResult DecimalSumValue(DecimalConstPtr values, usize len, DecimalRoundingAbi rounding, uint flags) {
    return SumCore(values, len, rounding, flags, false);
}
@export("chic_rt_decimal_sum") public static DecimalRuntimeResult chic_rt_decimal_sum(DecimalConstPtr values,
usize len, DecimalRoundingAbi rounding, uint flags) {
    return DecimalSumValue(values, len, rounding, flags);
}
@export("chic_rt_decimal_sum_out") public static void chic_rt_decimal_sum_out(* mut DecimalRuntimeResult outPtr,
DecimalConstPtr values, usize len, DecimalRoundingAbi rounding, uint flags) {
    WriteResult(outPtr, DecimalSumValue(values, len, rounding, flags));
}
private static DecimalRuntimeResult DecimalDotValue(DecimalConstPtr lhs, DecimalConstPtr rhs, usize len, DecimalRoundingAbi rounding,
uint flags) {
    return DotCore(lhs, rhs, len, rounding, flags, false);
}
@export("chic_rt_decimal_dot") public static DecimalRuntimeResult chic_rt_decimal_dot(DecimalConstPtr lhs, DecimalConstPtr rhs,
usize len, DecimalRoundingAbi rounding, uint flags) {
    return DecimalDotValue(lhs, rhs, len, rounding, flags);
}
@export("chic_rt_decimal_dot_out") public static void chic_rt_decimal_dot_out(* mut DecimalRuntimeResult outPtr,
DecimalConstPtr lhs, DecimalConstPtr rhs, usize len, DecimalRoundingAbi rounding, uint flags) {
    WriteResult(outPtr, DecimalDotValue(lhs, rhs, len, rounding, flags));
}
@export("chic_rt_decimal_matmul") public static int chic_rt_decimal_matmul(DecimalConstPtr left, usize leftRows,
usize leftCols, DecimalConstPtr right, usize rightCols, DecimalMutPtr dest, DecimalRoundingAbi rounding, uint flags) {
    return MatMulCore(left, leftRows, leftCols, right, rightCols, dest, rounding, flags, false);
}

public unsafe static void DecimalTestCoverageHelpers() {
    let _ = Zero();
    let _ = IsNullConstParts((* const @readonly Decimal128Parts) NativePtr.NullConst());
    let _ = IsNullMutParts((* mut Decimal128Parts) NativePtr.NullMut());
    let _ = IsNullResultPtr((* mut DecimalRuntimeResult) NativePtr.NullMut());
    var one = new Decimal128Parts {
        lo = 1u32, mid = 0u32, hi = 0u32, flags = 0u32
    }
    ;
    var two = new Decimal128Parts {
        lo = 2u32, mid = 0u32, hi = 0u32, flags = 0u32
    }
    ;
    let rounding = new DecimalRoundingAbi {
        value = 0u32
    }
    ;
    var resultValue = DecZero();
    let _ = BinaryOpValue(& one, & two, rounding, 0u, false, false, DecimalBinaryKind.Add, out resultValue);
    let _ = BinaryOp(& one, & two, rounding, 0u, false, true, DecimalBinaryKind.Div);
    let _ = TernaryOp(& one, & two, & one, rounding, 0u, false, DecimalTernaryKind.Fma);
    let _ = SumCore(new DecimalConstPtr {
        Pointer = & one
    }
    , 1usize, rounding, 0u, false);
    let _ = DotCore(new DecimalConstPtr {
        Pointer = & one
    }
    , new DecimalConstPtr {
        Pointer = & two
    }
    , 1usize, rounding, 0u, false);
    var dest = new Decimal128Parts {
        lo = 0u32, mid = 0u32, hi = 0u32, flags = 0u32
    }
    ;
    let _ = MatMulCore(new DecimalConstPtr {
        Pointer = & one
    }
    , 1usize, 1usize, new DecimalConstPtr {
        Pointer = & two
    }
    , 1usize, new DecimalMutPtr {
        Pointer = & dest
    }
    , rounding, 0u, false);
    let _ = ActiveRounding();
    let _ = HandleRounding(DecimalRoundingMode.TowardZero);
    var powValue = 0i128;
    let _ = TryPow10(2u32, out powValue);
    let _ = RoundQuotient(5i128, 2i128, DecimalRoundingMode.TiesToEven);
    let _ = ValidateFlags(0u, false);
    var roundMode = DecimalRoundingMode.TiesToEven;
    let _ = TryDecodeRounding(rounding, out roundMode);
    var local = 0i32;
    let _ = LocalI32Const(ref local);
    let _ = LocalPartsConst(ref one);
    let _ = PartsConst(& one);
    let _ = PartsMut(& dest);
    let _ = PartsMutFromPtr(& dest);
    let _ = ToParts(FromParts(one));
    let _ = MakeSimpleBinaryResult(& one, & two, DecimalBinaryKind.Sub, false);
    var scaledLeft = Compose(12u128, 0u32, false);
    var scaledRight = Compose(34u128, 2u32, false);
    let _ = ScaleUp(ref scaledLeft, 1u32);
    var leftOut = DecZero();
    var rightOut = DecZero();
    var scale = 0u32;
    let _ = AlignScales(scaledLeft, scaledRight, out leftOut, out rightOut, out scale);
    var addResult = DecZero();
    let _ = TryAdd(scaledLeft, scaledRight, out addResult);
    var subResult = DecZero();
    let _ = TrySub(scaledLeft, scaledRight, out subResult);
    var mulResult = DecZero();
    let _ = TryMul(scaledLeft, scaledRight, out mulResult);
    var divResult = DecZero();
    let _ = TryDiv(scaledLeft, scaledRight, out divResult);
    var remResult = DecZero();
    let _ = TryRem(scaledLeft, scaledRight, out remResult);
    var fmaResult = DecZero();
    let _ = TryFma(scaledLeft, scaledRight, scaledRight, out fmaResult);
    let decoded = DecodeParts(one);
    let encoded = EncodeDec(decoded);
    let _ = Magnitude(decoded);
    let _ = SignedCoeff(decoded);
    var result = new DecimalRuntimeResult {
        status = 0, value = encoded
    }
    ;
    WriteResult(& result, MakeResult(DecimalRuntimeStatus.Success, decoded));
    let _ = SignedCoeff(FromSigned(- 3i128, 0u32));
    let _ = Success(FromSigned(1i128, 0u32));
    let _ = Failure(DecimalRuntimeStatus.InvalidOperand);
    let _ = SuccessZero();
}
