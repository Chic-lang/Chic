namespace Std.Security.Cryptography;
import Std.Numeric;
import Std.Span;
/// <summary>Curve25519 Montgomery ladder for TLS key exchange.</summary>
public static class X25519
{
    private const int KeySize = 32;
    public const int PublicKeySize = KeySize;
    public const int SharedSecretSize = KeySize;
    public static X25519KeyPair GenerateKeyPair() {
        var privateKey = RandomNumberGenerator.GetBytes(KeySize);
        var publicKey = new byte[KeySize];
        return new X25519KeyPair(publicKey, privateKey);
    }
    public static int ComputePublicKey(ReadOnlySpan <byte >privateKey, Span <byte >publicKey) {
        if (publicKey.Length <KeySize)
        {
            throw new Std.ArgumentException("destination too small for public key");
        }
        var scalar = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(KeySize));
        scalar.Fill(0u8);
        let toCopy = privateKey.Length <NumericUnchecked.ToUSize(KeySize) ?privateKey.Length : NumericUnchecked.ToUSize(KeySize);
        scalar.Slice(0usize, toCopy).CopyFrom(privateKey.Slice(0usize, toCopy));
        var basePoint = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(KeySize));
        basePoint.Fill(0u8);
        basePoint[0usize] = 9u8;
        Curve25519(publicKey, scalar.AsReadOnly(), basePoint.AsReadOnly());
        return KeySize;
    }
    public static int ComputeSharedSecret(ReadOnlySpan <byte >privateKey, ReadOnlySpan <byte >peerPublicKey, Span <byte >sharedSecret) {
        if (sharedSecret.Length <KeySize)
        {
            throw new Std.ArgumentException("destination too small for shared secret");
        }
        if (privateKey.Length <NumericUnchecked.ToUSize (KeySize) || peerPublicKey.Length <NumericUnchecked.ToUSize (KeySize))
        {
            throw new Std.ArgumentException("invalid key length");
        }
        var scalar = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(KeySize));
        scalar.CopyFrom(privateKey.Slice(0usize, NumericUnchecked.ToUSize(KeySize)));
        var peer = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(KeySize));
        peer.CopyFrom(peerPublicKey.Slice(0usize, NumericUnchecked.ToUSize(KeySize)));
        var output = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(KeySize));
        Curve25519(output, scalar.AsReadOnly(), peer.AsReadOnly());
        sharedSecret.Slice(0usize, NumericUnchecked.ToUSize(KeySize)).CopyFrom(output.AsReadOnly());
        return KeySize;
    }
    private static void Clamp(Span <byte >scalar) {
        scalar[0] = (byte)(scalar[0] & 248u8);
        scalar[31usize] = (byte)((scalar[31usize] & 127u8) | 64u8);
    }
    private static void Curve25519(Span <byte >output, ReadOnlySpan <byte >scalar, ReadOnlySpan <byte >point) {
        var e = Span <byte >.StackAlloc(KeySize);
        e.CopyFrom(scalar);
        Clamp(e);
        var bp = new byte[KeySize];
        Span <byte >.FromArray(ref bp).Slice(0usize, KeySize).CopyFrom(point);
        var x = new long[10];
        var z = new long[10];
        var bpLimbs = new long[10];
        var zmone = new long[10];
        FExpand(bpLimbs, ReadOnlySpan <byte >.FromArray(ref bp));
        CMult(x, z, ReadOnlySpan <byte >.FromArray(ref e), bpLimbs);
        CRecip(zmone, z);
        var prod = new long[10];
        FMul(prod, x, zmone);
        FContract(output, prod);
    }
    private static void FSum(long[] output, long[] input) {
        var i = 0;
        while (i <10)
        {
            output[i] = output[i] + input[i];
            output[i + 1] = output[i + 1] + input[i + 1];
            i += 2;
        }
    }
    private static void FDifference(long[] output, long[] input) {
        var i = 0;
        while (i <10)
        {
            output[i] = input[i] - output[i];
            i += 1;
        }
    }
    private static void FScalarProduct(long[] output, long[] input, long scalar) {
        var i = 0;
        while (i <10)
        {
            output[i] = input[i] * scalar;
            i += 1;
        }
    }
    private static void FProduct(long[] output, long[] in2, long[] input) {
        output[0] = input[0] * in2[0];
        output[1] = input[1] * in2[0] + input[0] * in2[1];
        output[2] = 2l * input[1] * in2[1] + input[0] * in2[2] + input[2] * in2[0];
        output[3] = input[2] * in2[1] + input[1] * in2[2] + input[0] * in2[3] + input[3] * in2[0];
        output[4] = input[2] * in2[2] + 2l * (input[1] * in2[3] + input[3] * in2[1]) + input[0] * in2[4] + input[4] * in2[0];
        output[5] = input[3] * in2[2] + input[2] * in2[3] + input[4] * in2[1] + input[1] * in2[4] + input[0] * in2[5] + input[5] * in2[0];
        output[6] = 2l * (input[3] * in2[3] + input[1] * in2[5] + input[5] * in2[1]) + input[2] * in2[4] + input[4] * in2[2] + input[0] * in2[6] + input[6] * in2[0];
        output[7] = input[3] * in2[4] + input[4] * in2[3] + input[2] * in2[5] + input[5] * in2[2] + input[1] * in2[6] + input[6] * in2[1] + input[0] * in2[7] + input[7] * in2[0];
        output[8] = input[4] * in2[4] + 2l * (input[3] * in2[5] + input[5] * in2[3] + input[1] * in2[7] + input[7] * in2[1]) + input[2] * in2[6] + input[6] * in2[2] + input[0] * in2[8] + input[8] * in2[0];
        output[9] = input[4] * in2[5] + input[5] * in2[4] + input[3] * in2[6] + input[6] * in2[3] + input[2] * in2[7] + input[7] * in2[2] + input[1] * in2[8] + input[8] * in2[1] + input[0] * in2[9] + input[9] * in2[0];
        output[10] = 2l * (input[5] * in2[5] + input[3] * in2[7] + input[7] * in2[3] + input[1] * in2[9] + input[9] * in2[1]) + input[4] * in2[6] + input[6] * in2[4] + input[2] * in2[8] + input[8] * in2[2];
        output[11] = input[5] * in2[6] + input[6] * in2[5] + input[4] * in2[7] + input[7] * in2[4] + input[3] * in2[8] + input[8] * in2[3] + input[2] * in2[9] + input[9] * in2[2];
        output[12] = input[6] * in2[6] + 2l * (input[5] * in2[7] + input[7] * in2[5] + input[3] * in2[9] + input[9] * in2[3]) + input[4] * in2[8] + input[8] * in2[4];
        output[13] = input[6] * in2[7] + input[7] * in2[6] + input[5] * in2[8] + input[8] * in2[5] + input[4] * in2[9] + input[9] * in2[4];
        output[14] = 2l * (input[7] * in2[7] + input[5] * in2[9] + input[9] * in2[5]) + input[6] * in2[8] + input[8] * in2[6];
        output[15] = input[7] * in2[8] + input[8] * in2[7] + input[6] * in2[9] + input[9] * in2[6];
        output[16] = input[8] * in2[8] + 2l * (input[7] * in2[9] + input[9] * in2[7]);
        output[17] = input[8] * in2[9] + input[9] * in2[8];
        output[18] = 2l * input[9] * in2[9];
    }
    private static void FReduceDegree(long[] output) {
        output[8] += (output[18] << 4);
        output[8] += (output[18] << 1);
        output[8] += output[18];
        output[7] += (output[17] << 4);
        output[7] += (output[17] << 1);
        output[7] += output[17];
        output[6] += (output[16] << 4);
        output[6] += (output[16] << 1);
        output[6] += output[16];
        output[5] += (output[15] << 4);
        output[5] += (output[15] << 1);
        output[5] += output[15];
        output[4] += (output[14] << 4);
        output[4] += (output[14] << 1);
        output[4] += output[14];
        output[3] += (output[13] << 4);
        output[3] += (output[13] << 1);
        output[3] += output[13];
        output[2] += (output[12] << 4);
        output[2] += (output[12] << 1);
        output[2] += output[12];
        output[1] += (output[11] << 4);
        output[1] += (output[11] << 1);
        output[1] += output[11];
        output[0] += (output[10] << 4);
        output[0] += (output[10] << 1);
        output[0] += output[10];
    }
    private static long DivBy2Pow26(long v) {
        let high = (uint)(NumericUnchecked.ToUInt64(v) >> 32);
        let sign = ((int) high) >> 31;
        let round = NumericUnchecked.ToInt32(((uint) sign) >> 6);
        return(v + round) >> 26;
    }
    private static long DivBy2Pow25(long v) {
        let high = (uint)(NumericUnchecked.ToUInt64(v) >> 32);
        let sign = ((int) high) >> 31;
        let round = NumericUnchecked.ToInt32(((uint) sign) >> 7);
        return(v + round) >> 25;
    }
    private static void FReduceCoefficients(long[] output) {
        var i = 0;
        output[10] = 0l;
        while (i <10)
        {
            let over = DivBy2Pow26(output[i]);
            output[i] -= (over << 26);
            output[i + 1] += over;
            let overNext = DivBy2Pow25(output[i + 1]);
            output[i + 1] -= (overNext << 25);
            output[i + 2] += overNext;
            i += 2;
        }
        output[0] += (output[10] << 4);
        output[0] += (output[10] << 1);
        output[0] += output[10];
        output[10] = 0l;
        let finalCarry = DivBy2Pow26(output[0]);
        output[0] -= (finalCarry << 26);
        output[1] += finalCarry;
    }
    private static void FMul(long[] output, long[] left, long[] right) {
        var t = new long[19];
        FProduct(t, left, right);
        FReduceDegree(t);
        FReduceCoefficients(t);
        var idx = 0;
        while (idx <10)
        {
            output[idx] = t[idx];
            idx += 1;
        }
    }
    private static void FSquareInner(long[] output, long[] input) {
        output[0] = input[0] * input[0];
        output[1] = 2l * input[0] * input[1];
        output[2] = 2l * (input[1] * input[1] + input[0] * input[2]);
        output[3] = 2l * (input[1] * input[2] + input[0] * input[3]);
        output[4] = input[2] * input[2] + 4l * input[1] * input[3] + 2l * input[0] * input[4];
        output[5] = 2l * (input[2] * input[3] + input[1] * input[4] + input[0] * input[5]);
        output[6] = 2l * (input[3] * input[3] + input[2] * input[4] + input[0] * input[6]) + 4l * input[1] * input[5];
        output[7] = 2l * (input[3] * input[4] + input[2] * input[5] + input[1] * input[6] + input[0] * input[7]);
        output[8] = input[4] * input[4] + 2l * (input[2] * input[6] + input[0] * input[8]) + 4l * (input[1] * input[7] + input[3] * input[5]);
        output[9] = 2l * (input[4] * input[5] + input[3] * input[6] + input[2] * input[7] + input[1] * input[8] + input[0] * input[9]);
        output[10] = 2l * (input[5] * input[5] + input[3] * input[7] + input[1] * input[9]) + 4l * (input[2] * input[8] + input[4] * input[6]);
        output[11] = 2l * (input[5] * input[6] + input[4] * input[7] + input[3] * input[8] + input[2] * input[9]);
        output[12] = input[6] * input[6] + 4l * (input[3] * input[9] + input[5] * input[7]) + 2l * input[4] * input[8];
        output[13] = 2l * (input[6] * input[7] + input[5] * input[8] + input[4] * input[9]);
        output[14] = 2l * input[7] * input[7] + 4l * (input[5] * input[9]) + 2l * input[6] * input[8];
        output[15] = 2l * (input[7] * input[8] + input[6] * input[9]);
        output[16] = input[8] * input[8] + 4l * input[7] * input[9];
        output[17] = 2l * input[8] * input[9];
        output[18] = 2l * input[9] * input[9];
    }
    private static void FSquareTimes(long[] output, long[] input, int count) {
        var t = new long[19];
        var i = 0;
        FSquareInner(output, input);
        while (i <count)
        {
            FReduceDegree(output);
            FReduceCoefficients(output);
            FSquareInner(t, output);
            FReduceDegree(t);
            FReduceCoefficients(t);
            var idx = 0;
            while (idx <19)
            {
                output[idx] = t[idx];
                idx += 1;
            }
            i += 1;
        }
    }
    private static void FSquare(long[] output, long[] input) {
        var t = new long[19];
        FSquareInner(t, input);
        FReduceDegree(t);
        FReduceCoefficients(t);
        var i = 0;
        while (i <10)
        {
            output[i] = t[i];
            i += 1;
        }
    }
    private static void FContract(Span <byte >output, long[] inputLimbs) {
        var input = new int[10];
        var i = 0;
        while (i <10)
        {
            input[i] = NumericUnchecked.ToInt32(inputLimbs[i]);
            i += 1;
        }
        var j = 0;
        while (j <2)
        {
            i = 0;
            while (i <9)
            {
                if ( (i & 1) == 1)
                {
                    let mask = input[i] >> 31;
                    let carry = - ((input[i] & mask) >> 25);
                    input[i] = input[i] + (carry << 25);
                    input[i + 1] = input[i + 1] - carry;
                }
                else
                {
                    let mask = input[i] >> 31;
                    let carry = - ((input[i] & mask) >> 26);
                    input[i] = input[i] + (carry << 26);
                    input[i + 1] = input[i + 1] - carry;
                }
                i += 1;
            }
            let mask9 = input[9] >> 31;
            let carry9 = - ((input[9] & mask9) >> 25);
            input[9] = input[9] + (carry9 << 25);
            input[0] = input[0] - (carry9 * 19);
            j += 1;
        }
        let mask0 = input[0] >> 31;
        let carry0 = - ((input[0] & mask0) >> 26);
        input[0] = input[0] + (carry0 << 26);
        input[1] = input[1] - carry0;
        j = 0;
        while (j <2)
        {
            i = 0;
            while (i <9)
            {
                if ( (i & 1) == 1)
                {
                    let carry = input[i] >> 25;
                    input[i] &= 0x1ffffff;
                    input[i + 1] += carry;
                }
                else
                {
                    let carry = input[i] >> 26;
                    input[i] &= 0x3ffffff;
                    input[i + 1] += carry;
                }
                i += 1;
            }
            let carryLast = input[9] >> 25;
            input[9] &= 0x1ffffff;
            input[0] += 19 * carryLast;
            j += 1;
        }
        var mask = S32Gte(input[0], 0x3ffffed);
        i = 1;
        while (i <10)
        {
            if ( (i & 1) == 1)
            {
                mask &= S32Eq(input[i], 0x1ffffff);
            }
            else
            {
                mask &= S32Eq(input[i], 0x3ffffff);
            }
            i += 1;
        }
        input[0] -= mask & 0x3ffffed;
        i = 1;
        while (i <10)
        {
            if ( (i & 1) == 1)
            {
                input[i] -= mask & 0x1ffffff;
            }
            else
            {
                input[i] -= mask & 0x3ffffff;
            }
            i += 1;
        }
        input[1] <<= 2;
        input[2] <<= 3;
        input[3] <<= 5;
        input[4] <<= 6;
        input[6] <<= 1;
        input[7] <<= 3;
        input[8] <<= 4;
        input[9] <<= 6;
        var clearIdx = 0usize;
        while (clearIdx <NumericUnchecked.ToUSize (output.Length))
        {
            output[clearIdx] = 0u8;
            clearIdx += 1usize;
        }
        WriteLimb(output, 0usize, input[0]);
        WriteLimb(output, 3usize, input[1]);
        WriteLimb(output, 6usize, input[2]);
        WriteLimb(output, 9usize, input[3]);
        WriteLimb(output, 12usize, input[4]);
        WriteLimb(output, 16usize, input[5]);
        WriteLimb(output, 19usize, input[6]);
        WriteLimb(output, 22usize, input[7]);
        WriteLimb(output, 25usize, input[8]);
        WriteLimb(output, 28usize, input[9]);
    }
    private static void WriteLimb(Span <byte >output, usize offset, int limb) {
        output[offset] = NumericUnchecked.ToByte(limb & 0xFF);
        output[offset + 1usize] = NumericUnchecked.ToByte((limb >> 8) & 0xFF);
        output[offset + 2usize] = NumericUnchecked.ToByte((limb >> 16) & 0xFF);
        output[offset + 3usize] = NumericUnchecked.ToByte((limb >> 24) & 0xFF);
    }
    private static int S32Eq(int a, int b) {
        a = ~ (a ^ b);
        a &= a << 16;
        a &= a << 8;
        a &= a << 4;
        a &= a << 2;
        a &= a << 1;
        return a >> 31;
    }
    private static int S32Gte(int a, int b) {
        a -= b;
        return ~ (a >> 31);
    }
    private static void FExpand(long[] output, ReadOnlySpan <byte >input) {
        var t = new byte[32];
        Span <byte >.FromArray(ref t).CopyFrom(input.Slice(0usize, 32usize));
        output[0] = (((long) t[0usize]) | ((long) t[1usize] << 8) | ((long) t[2usize] << 16) | ((long) t[3usize] << 24)) & 0x3ffffffl;
        output[1] = ((((long) t[3usize]) | ((long) t[4usize] << 8) | ((long) t[5usize] << 16) | ((long) t[6usize] << 24)) >> 2) & 0x1ffffffl;
        output[2] = ((((long) t[6usize]) | ((long) t[7usize] << 8) | ((long) t[8usize] << 16) | ((long) t[9usize] << 24)) >> 3) & 0x3ffffffl;
        output[3] = ((((long) t[9usize]) | ((long) t[10usize] << 8) | ((long) t[11usize] << 16) | ((long) t[12usize] << 24)) >> 5) & 0x1ffffffl;
        output[4] = ((((long) t[12usize]) | ((long) t[13usize] << 8) | ((long) t[14usize] << 16) | ((long) t[15usize] << 24)) >> 6) & 0x3ffffffl;
        output[5] = (((long) t[16usize]) | ((long) t[17usize] << 8) | ((long) t[18usize] << 16) | ((long) t[19usize] << 24)) & 0x1ffffffl;
        output[6] = ((((long) t[19usize]) | ((long) t[20usize] << 8) | ((long) t[21usize] << 16) | ((long) t[22usize] << 24)) >> 1) & 0x3ffffffl;
        output[7] = ((((long) t[22usize]) | ((long) t[23usize] << 8) | ((long) t[24usize] << 16) | ((long) t[25usize] << 24)) >> 3) & 0x1ffffffl;
        output[8] = ((((long) t[25usize]) | ((long) t[26usize] << 8) | ((long) t[27usize] << 16) | ((long) t[28usize] << 24)) >> 4) & 0x3ffffffl;
        output[9] = ((((long) t[28usize]) | ((long) t[29usize] << 8) | ((long) t[30usize] << 16) | ((long) t[31usize] << 24)) >> 6) & 0x1ffffffl;
    }
    private static void SwapConditional(long[] a, long[] b, long iswap) {
        var swap = - (int) iswap;
        var i = 0;
        while (i <10)
        {
            let x = swap & (NumericUnchecked.ToInt32(a[i]) ^ NumericUnchecked.ToInt32(b[i]));
            a[i] = NumericUnchecked.ToInt32(a[i]) ^ x;
            b[i] = NumericUnchecked.ToInt32(b[i]) ^ x;
            i += 1;
        }
    }
    private static void FMonty(long[] x2, long[] z2, long[] x3, long[] z3, long[] x, long[] z, long[] xprime, long[] zprime,
    long[] qmqp) {
        var origx = new long[10];
        var origxprime = new long[10];
        var zzz = new long[19];
        var xx = new long[19];
        var zz = new long[19];
        var xxprime = new long[19];
        var zzprime = new long[19];
        var zzzprime = new long[19];
        var xxxprime = new long[19];
        var i = 0;
        while (i <10)
        {
            origx[i] = x[i];
            origxprime[i] = xprime[i];
            i += 1;
        }
        FSum(x, z);
        FDifference(z, origx);
        FSum(xprime, zprime);
        FDifference(zprime, origxprime);
        FProduct(xxprime, xprime, z);
        FProduct(zzprime, x, zprime);
        FReduceDegree(xxprime);
        FReduceCoefficients(xxprime);
        FReduceDegree(zzprime);
        FReduceCoefficients(zzprime);
        i = 0;
        while (i <10)
        {
            origxprime[i] = xxprime[i];
            i += 1;
        }
        FSum(xxprime, zzprime);
        FDifference(zzprime, origxprime);
        FSquare(xxxprime, xxprime);
        FSquare(zzzprime, zzprime);
        FProduct(zzprime, zzzprime, qmqp);
        FReduceDegree(zzprime);
        FReduceCoefficients(zzprime);
        i = 0;
        while (i <10)
        {
            x3[i] = xxxprime[i];
            z3[i] = zzprime[i];
            i += 1;
        }
        FSquare(xx, x);
        FSquare(zz, z);
        FProduct(x2, xx, zz);
        FReduceDegree(x2);
        FReduceCoefficients(x2);
        FDifference(zz, xx);
        var idx = 10;
        while (idx <19)
        {
            zzz[idx] = 0l;
            idx += 1;
        }
        FScalarProduct(zzz, zz, 121665l);
        FReduceCoefficients(zzz);
        FSum(zzz, xx);
        FProduct(z2, zz, zzz);
        FReduceDegree(z2);
        FReduceCoefficients(z2);
    }
    private static void CMult(long[] resultx, long[] resultz, ReadOnlySpan <byte >n, long[] q) {
        var a = new long[19];
        var b = new long[19];
        var c = new long[19];
        var d = new long[19];
        var e = new long[19];
        var f = new long[19];
        var g = new long[19];
        var h = new long[19];
        var nqpqx = a;
        var nqpqz = b;
        var nqx = c;
        var nqz = d;
        var nqpqx2 = e;
        var nqpqz2 = f;
        var nqx2 = g;
        var nqz2 = h;
        var i = 0;
        while (i <10)
        {
            nqpqx[i] = q[i];
            i += 1;
        }
        nqpqz[0] = 1l;
        nqx[0] = 1l;
        nqpqz2[0] = 1l;
        nqz2[0] = 1l;
        i = 0;
        while (i <32)
        {
            let bVal = n[31usize - i];
            var j = 0;
            while (j <8)
            {
                let bit = (long)(bVal >> 7);
                SwapConditional(nqx, nqpqx, bit);
                SwapConditional(nqz, nqpqz, bit);
                FMonty(nqx2, nqz2, nqpqx2, nqpqz2, nqx, nqz, nqpqx, nqpqz, q);
                SwapConditional(nqx2, nqpqx2, bit);
                SwapConditional(nqz2, nqpqz2, bit);
                var t = nqx;
                nqx = nqx2;
                nqx2 = t;
                t = nqz;
                nqz = nqz2;
                nqz2 = t;
                t = nqpqx;
                nqpqx = nqpqx2;
                nqpqx2 = t;
                t = nqpqz;
                nqpqz = nqpqz2;
                nqpqz2 = t;
                j += 1;
                bVal = NumericUnchecked.ToByte((bVal << 1) & 0xFFu8);
            }
            i += 1;
        }
        i = 0;
        while (i <10)
        {
            resultx[i] = nqx[i];
            resultz[i] = nqz[i];
            i += 1;
        }
    }
    private static void CRecip(long[] outLimbs, long[] z) {
        var z2 = new long[19];
        var z9 = new long[19];
        var z11 = new long[19];
        var z2_5_0 = new long[19];
        var z2_10_0 = new long[19];
        var z2_20_0 = new long[19];
        var z2_50_0 = new long[19];
        var z2_100_0 = new long[19];
        var t0 = new long[19];
        var t1 = new long[19];
        FSquare(z2, z);
        FSquare(t1, z2);
        FSquare(t0, t1);
        FMul(z9, t0, z);
        FMul(z11, z9, z2);
        FSquare(t0, z11);
        FMul(z2_5_0, t0, z9);
        FSquare(t0, z2_5_0);
        FSquare(t1, t0);
        FSquare(t0, t1);
        FSquare(t1, t0);
        FSquare(t0, t1);
        FMul(z2_10_0, t0, z2_5_0);
        FSquare(t0, z2_10_0);
        FSquare(t1, t0);
        var i = 2;
        while (i <10)
        {
            FSquare(t0, t1);
            FSquare(t1, t0);
            i += 2;
        }
        FMul(z2_20_0, t1, z2_10_0);
        FSquare(t0, z2_20_0);
        FSquare(t1, t0);
        i = 2;
        while (i <20)
        {
            FSquare(t0, t1);
            FSquare(t1, t0);
            i += 2;
        }
        FMul(t0, t1, z2_20_0);
        FSquare(t1, t0);
        FSquare(t0, t1);
        i = 2;
        while (i <10)
        {
            FSquare(t1, t0);
            FSquare(t0, t1);
            i += 2;
        }
        FMul(z2_50_0, t0, z2_10_0);
        FSquare(t0, z2_50_0);
        FSquare(t1, t0);
        i = 2;
        while (i <50)
        {
            FSquare(t0, t1);
            FSquare(t1, t0);
            i += 2;
        }
        FMul(z2_100_0, t1, z2_50_0);
        FSquare(t1, z2_100_0);
        FSquare(t0, t1);
        i = 2;
        while (i <100)
        {
            FSquare(t1, t0);
            FSquare(t0, t1);
            i += 2;
        }
        FMul(t1, t0, z2_100_0);
        FSquare(t0, t1);
        FSquare(t1, t0);
        i = 2;
        while (i <50)
        {
            FSquare(t0, t1);
            FSquare(t1, t0);
            i += 2;
        }
        FMul(t0, t1, z2_50_0);
        FSquare(t1, t0);
        FSquare(t0, t1);
        FSquare(t1, t0);
        FSquare(t0, t1);
        FSquare(t1, t0);
        FMul(outLimbs, t1, z11);
    }
}
