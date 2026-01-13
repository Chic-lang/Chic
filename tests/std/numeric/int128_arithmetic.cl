namespace Exec;

import Std;
import Std.Numeric;

testcase Int128OperationsAreCorrect()
{
    if (Int128.Sign(new Int128(-5)) != -1)
    {
        return false;
    }
    if (Int128.Sign(Int128.Zero) != 0)
    {
        return false;
    }
    if (Int128.Sign(new Int128(9)) != 1)
    {
        return false;
    }

    var remainder = new Int128(0);
    let quotient = Int128.DivRem(new Int128(1234), new Int128(37), out remainder);
    if (quotient != new Int128(33) || remainder != new Int128(13))
    {
        return false;
    }

    let clampedLow = Int128.Clamp(new Int128(-10), new Int128(-5), new Int128(5));
    if (clampedLow != new Int128(-5))
    {
        return false;
    }
    let clampedHigh = Int128.Clamp(new Int128(20), new Int128(-5), new Int128(5));
    if (clampedHigh != new Int128(5))
    {
        return false;
    }

    let maxMag = Int128.MaxMagnitude(new Int128(-50), new Int128(40));
    if (maxMag != new Int128(-50))
    {
        return false;
    }
    let minMag = Int128.MinMagnitude(new Int128(-50), new Int128(40));
    if (minMag != new Int128(40))
    {
        return false;
    }

    let copyNegative = Int128.CopySign(new Int128(12), new Int128(-3));
    if (copyNegative != new Int128(-12))
    {
        return false;
    }
    let copyPositive = Int128.CopySign(new Int128(-12), new Int128(7));
    if (copyPositive != new Int128(12))
    {
        return false;
    }

    let absMin = Int128.Abs(new Int128(Int128.MinValue));
    if (absMin != new Int128(Int128.MinValue))
    {
        return false;
    }

    return true;
}

testcase Int128ConversionsRoundAndCheck()
{
    let fromFloat = new Int128(12.75f);
    if (fromFloat != new Int128(12))
    {
        return false;
    }

    let fromDouble = new Int128(98765.0d);
    if (fromDouble != new Int128(98765))
    {
        return false;
    }

    let fromNegative = new Int128(-2.9d);
    if (fromNegative != new Int128(-2))
    {
        return false;
    }

    var overflowed = false;
    try
    {
        let _ = new Int128(1.0e50d);
    }
    catch (Std.OverflowException)
    {
        overflowed = true;
    }
    if (!overflowed)
    {
        return false;
    }

    let uFromFloat = new UInt128(42.5f);
    if (uFromFloat != new UInt128(42u128))
    {
        return false;
    }

    var unsignedNegative = false;
    try
    {
        let _ = new UInt128(-5.0d);
    }
    catch (Std.OverflowException)
    {
        unsignedNegative = true;
    }
    return unsignedNegative;
}
