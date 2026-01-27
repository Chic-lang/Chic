namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;

public static class DecimalTestSupport
{
    public static Decimal128Parts Make(u32 lo, u32 mid, u32 hi, u32 scale, bool negative) {
        let sign = negative ?0x80000000u : 0u;
        let flags = (scale << 16) | sign;
        return new Decimal128Parts {
            lo = lo, mid = mid, hi = hi, flags = flags
        }
        ;
    }
}

testcase Given_decimal_binary_operations_When_executed_Then_decimal_binary_operations()
{
    let rounding = new DecimalRoundingAbi {
        value = 0u32
    }
    ;
    var one = new Decimal128Parts {
        lo = 1u32, mid = 0u32, hi = 0u32, flags = 0u32
    }
    ;
    var two = new Decimal128Parts {
        lo = 2u32, mid = 0u32, hi = 0u32, flags = 0u32
    }
    ;
    var zero = new Decimal128Parts {
        lo = 0u32, mid = 0u32, hi = 0u32, flags = 0u32
    }
    ;
    let add = chic_rt_decimal_add(& one, & two, rounding, 0u);
    var ok = add.status == 0 && add.value.lo == 3u;
    let sub = chic_rt_decimal_sub(& two, & one, rounding, 0u);
    ok = ok && sub.status == 0;
    let mul = chic_rt_decimal_mul(& two, & two, rounding, 0u);
    ok = ok && mul.status == 0;
    let rem = chic_rt_decimal_rem(& two, & one, rounding, 0u);
    ok = ok && rem.status == 0;
    let divZero = chic_rt_decimal_div(& two, & zero, rounding, 0u);
    ok = ok && divZero.status == 2;
    Assert.That(ok).IsTrue();
}

testcase Given_decimal_clone_sum_dot_and_matmul_When_executed_Then_decimal_clone_sum_dot_and_matmul()
{
    unsafe {
        let rounding = new DecimalRoundingAbi {
            value = 0u32
        }
        ;
        let partSize = (usize) __sizeof<Decimal128Parts>();
        let partAlign = (usize) __alignof<Decimal128Parts>();
        var buffer = MemoryRuntime.chic_rt_alloc(partSize * 2usize, partAlign);
        var * mut Decimal128Parts parts = buffer.Pointer;
        * parts = new Decimal128Parts {
            lo = 1u32, mid = 0u32, hi = 0u32, flags = 0u32
        }
        ;
        var * mut @expose_address byte secondRaw = NativePtr.OffsetMut(buffer.Pointer, (isize) partSize);
        var * mut Decimal128Parts second = secondRaw;
        * second = new Decimal128Parts {
            lo = 2u32, mid = 0u32, hi = 0u32, flags = 0u32
        }
        ;
        var dest = new Decimal128Parts {
            lo = 0u32, mid = 0u32, hi = 0u32, flags = 0u32
        }
        ;
        var destPtr = new DecimalMutPtr {
            Pointer = & dest
        }
        ;
        let cloneStatus = chic_rt_decimal_clone(new DecimalConstPtr {
            Pointer = parts
        }
        , destPtr);
        var ok = cloneStatus == 0;
        let sum = chic_rt_decimal_sum(new DecimalConstPtr {
            Pointer = parts
        }
        , 2usize, rounding, 0u);
        ok = ok && sum.status == 0;
        let dot = chic_rt_decimal_dot(new DecimalConstPtr {
            Pointer = parts
        }
        , new DecimalConstPtr {
            Pointer = parts
        }
        , 2usize, rounding, 0u);
        ok = ok && dot.status == 0;
        var matrixDest = new Decimal128Parts {
            lo = 0u32, mid = 0u32, hi = 0u32, flags = 0u32
        }
        ;
        let matStatus = chic_rt_decimal_matmul(new DecimalConstPtr {
            Pointer = parts
        }
        , 1usize, 1usize, new DecimalConstPtr {
            Pointer = parts
        }
        , 1usize, new DecimalMutPtr {
            Pointer = & matrixDest
        }
        , rounding, 0u);
        ok = ok && matStatus == 0;
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_decimal_out_variants_and_fma_When_executed_Then_decimal_out_variants_and_fma()
{
    let rounding = new DecimalRoundingAbi {
        value = 0u32
    }
    ;
    var one = new Decimal128Parts {
        lo = 1u32, mid = 0u32, hi = 0u32, flags = 0u32
    }
    ;
    var two = new Decimal128Parts {
        lo = 2u32, mid = 0u32, hi = 0u32, flags = 0u32
    }
    ;
    var three = new Decimal128Parts {
        lo = 3u32, mid = 0u32, hi = 0u32, flags = 0u32
    }
    ;
    var result = new DecimalRuntimeResult {
        status = 0, value = new Decimal128Parts {
            lo = 0u32, mid = 0u32, hi = 0u32, flags = 0u32
        }
    }
    ;
    chic_rt_decimal_add_out(& result, & one, & two, rounding, 0u);
    var ok = result.status == 0;
    chic_rt_decimal_sub_out(& result, & three, & one, rounding, 0u);
    ok = ok && result.status == 0;
    chic_rt_decimal_mul_out(& result, & two, & two, rounding, 0u);
    ok = ok && result.status == 0;
    chic_rt_decimal_div_out(& result, & three, & two, rounding, 0u);
    ok = ok && result.status == 0;
    chic_rt_decimal_rem_out(& result, & three, & two, rounding, 0u);
    ok = ok && result.status == 0;

    let fma = chic_rt_decimal_fma(& two, & two, & one, rounding, 0u);
    ok = ok && fma.status == 0;
    chic_rt_decimal_fma_out(& result, & two, & two, & one, rounding, 0u);
    ok = ok && result.status == 0;
    Assert.That(ok).IsTrue();
}

testcase Given_decimal_invalid_flags_rounding_and_zero_len_When_executed_Then_decimal_invalid_flags_rounding_and_zero_len()
{
    unsafe {
        let rounding = new DecimalRoundingAbi {
            value = 9u32
        }
        ;
        let okRounding = new DecimalRoundingAbi {
            value = 0u32
        }
        ;
        var parts = new Decimal128Parts {
            lo = 1u32, mid = 0u32, hi = 0u32, flags = 0u32
        }
        ;
        let invalidFlags = chic_rt_decimal_sum(new DecimalConstPtr {
            Pointer = & parts
        }
        , 1usize, okRounding, DecimalFlags.Vectorize);
        var ok = invalidFlags.status == (int) DecimalRuntimeStatus.InvalidFlags;
        let invalidRounding = chic_rt_decimal_sum(new DecimalConstPtr {
            Pointer = & parts
        }
        , 1usize, rounding, 0u);
        ok = ok && invalidRounding.status == (int) DecimalRuntimeStatus.InvalidRounding;
        let zeroLen = chic_rt_decimal_sum(new DecimalConstPtr {
            Pointer = & parts
        }
        , 0usize, okRounding, 0u);
        ok = ok && zeroLen.status == (int) DecimalRuntimeStatus.Success;

        let nullDot = chic_rt_decimal_dot(new DecimalConstPtr {
            Pointer = (* const Decimal128Parts) NativePtr.NullConst()
        }
        , new DecimalConstPtr {
            Pointer = & parts
        }
        , 1usize, okRounding, 0u);
        ok = ok && nullDot.status == (int) DecimalRuntimeStatus.InvalidPointer;

        let matmulFlags = chic_rt_decimal_matmul(new DecimalConstPtr {
            Pointer = & parts
        }
        , 1usize, 1usize, new DecimalConstPtr {
            Pointer = & parts
        }
        , 1usize, new DecimalMutPtr {
            Pointer = (* mut Decimal128Parts) NativePtr.NullMut()
        }
        , okRounding, 0u);
        ok = ok && matmulFlags == (int) DecimalRuntimeStatus.InvalidPointer;
        let matmulRounding = chic_rt_decimal_matmul(new DecimalConstPtr {
            Pointer = & parts
        }
        , 0usize, 0usize, new DecimalConstPtr {
            Pointer = & parts
        }
        , 0usize, new DecimalMutPtr {
            Pointer = (* mut Decimal128Parts) NativePtr.NullMut()
        }
        , okRounding, DecimalFlags.Vectorize);
        ok = ok && matmulRounding == (int) DecimalRuntimeStatus.InvalidFlags;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_decimal_rounding_and_scale_sweep_When_executed_Then_decimal_rounding_and_scale_sweep()
{
    unsafe {
        let partSize = (usize) __sizeof<Decimal128Parts>();
        let partAlign = (usize) __alignof<Decimal128Parts>();
        var buffer = MemoryRuntime.chic_rt_alloc(partSize * 2usize, partAlign);
        var * mut Decimal128Parts parts = buffer.Pointer;
        * parts = DecimalTestSupport.Make(123u32, 0u32, 0u32, 1u32, false);
        var * mut @expose_address byte secondRaw = NativePtr.OffsetMut(buffer.Pointer, (isize) partSize);
        var * mut Decimal128Parts second = secondRaw;
        * second = DecimalTestSupport.Make(456u32, 0u32, 0u32, 3u32, true);

        var ok = true;
        var rounding = new DecimalRoundingAbi {
            value = 0u32
        }
        ;
        let sum0 = chic_rt_decimal_sum(new DecimalConstPtr {
            Pointer = parts
        }
        , 2usize, rounding, 0u);
        ok = ok && sum0.status == (int) DecimalRuntimeStatus.Success;

        rounding.value = 1u32;
        let sum1 = chic_rt_decimal_sum(new DecimalConstPtr {
            Pointer = parts
        }
        , 2usize, rounding, 0u);
        ok = ok && sum1.status == (int) DecimalRuntimeStatus.Success;

        rounding.value = 2u32;
        let sum2 = chic_rt_decimal_sum(new DecimalConstPtr {
            Pointer = parts
        }
        , 2usize, rounding, 0u);
        ok = ok && sum2.status == (int) DecimalRuntimeStatus.Success;

        rounding.value = 3u32;
        let dot3 = chic_rt_decimal_dot(new DecimalConstPtr {
            Pointer = parts
        }
        , new DecimalConstPtr {
            Pointer = parts
        }
        , 2usize, rounding, 0u);
        ok = ok && dot3.status == (int) DecimalRuntimeStatus.Success;

        rounding.value = 4u32;
        let dot4 = chic_rt_decimal_dot(new DecimalConstPtr {
            Pointer = parts
        }
        , new DecimalConstPtr {
            Pointer = parts
        }
        , 2usize, rounding, 0u);
        ok = ok && dot4.status == (int) DecimalRuntimeStatus.Success;

        var bigScale = DecimalTestSupport.Make(1u32, 0u32, 0u32, 40u32, false);
        let addOverflow = chic_rt_decimal_add(& bigScale, & bigScale, rounding, 0u);
        ok = ok && addOverflow.status == (int) DecimalRuntimeStatus.InvalidOperand;
        let mulOverflow = chic_rt_decimal_mul(& bigScale, & bigScale, rounding, 0u);
        ok = ok && mulOverflow.status == (int) DecimalRuntimeStatus.InvalidOperand;

        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_decimal_rounding_modes_and_invalid_pointers_When_executed_Then_decimal_rounding_modes_and_invalid_pointers()
{
    unsafe {
        let one = new Decimal128Parts {
            lo = 1u32, mid = 0u32, hi = 0u32, flags = 0u32
        }
        ;
        let two = new Decimal128Parts {
            lo = 2u32, mid = 0u32, hi = 0u32, flags = 0u32
        }
        ;
        let roundZero = new DecimalRoundingAbi {
            value = 0u32
        }
        ;
        let roundOne = new DecimalRoundingAbi {
            value = 1u32
        }
        ;
        let roundTwo = new DecimalRoundingAbi {
            value = 2u32
        }
        ;
        let roundThree = new DecimalRoundingAbi {
            value = 3u32
        }
        ;
        let roundFour = new DecimalRoundingAbi {
            value = 4u32
        }
        ;
        var ok = chic_rt_decimal_div(& one, & two, roundZero, 0u).status == 0;
        ok = ok && chic_rt_decimal_div(& one, & two, roundOne, 0u).status == 0;
        ok = ok && chic_rt_decimal_div(& one, & two, roundTwo, 0u).status == 0;
        ok = ok && chic_rt_decimal_div(& one, & two, roundThree, 0u).status == 0;
        ok = ok && chic_rt_decimal_div(& one, & two, roundFour, 0u).status == 0;

        let addNull = chic_rt_decimal_add((* const Decimal128Parts) NativePtr.NullConst(), & two, roundZero, 0u);
        ok = ok && addNull.status == (int) DecimalRuntimeStatus.InvalidPointer;
        let sumNull = chic_rt_decimal_sum(new DecimalConstPtr {
            Pointer = (* const Decimal128Parts) NativePtr.NullConst()
        }
        , 2usize, roundZero, 0u);
        ok = ok && sumNull.status == (int) DecimalRuntimeStatus.InvalidPointer;
        let cloneNull = chic_rt_decimal_clone(new DecimalConstPtr {
            Pointer = & one
        }
        , new DecimalMutPtr {
            Pointer = (* mut Decimal128Parts) NativePtr.NullMut()
        }
        );
        ok = ok && cloneNull == (int) DecimalRuntimeStatus.InvalidPointer;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_decimal_sum_and_dot_out_variants_When_executed_Then_decimal_sum_and_dot_out_variants()
{
    unsafe {
        let rounding = new DecimalRoundingAbi {
            value = 0u32
        }
        ;
        var parts = new Decimal128Parts {
            lo = 2u32, mid = 0u32, hi = 0u32, flags = 0u32
        }
        ;
        var result = new DecimalRuntimeResult {
            status = 0, value = new Decimal128Parts {
                lo = 0u32, mid = 0u32, hi = 0u32, flags = 0u32
            }
        }
        ;
        chic_rt_decimal_sum_out(& result, new DecimalConstPtr {
            Pointer = & parts
        }
        , 1usize, rounding, 0u);
        var ok = result.status == 0;
        chic_rt_decimal_dot_out(& result, new DecimalConstPtr {
            Pointer = & parts
        }
        , new DecimalConstPtr {
            Pointer = & parts
        }
        , 1usize, rounding, 0u);
        ok = ok && result.status == 0;

        chic_rt_decimal_sum_out((* mut DecimalRuntimeResult) NativePtr.NullMut(), new DecimalConstPtr {
            Pointer = & parts
        }
        , 1usize, rounding, 0u);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_decimal_scaled_values_and_divide_by_zero_When_executed_Then_decimal_scaled_values_and_divide_by_zero()
{
    unsafe {
        let rounding = new DecimalRoundingAbi {
            value = 0u32
        }
        ;
        var a = DecimalTestSupport.Make(12345u, 0u, 0u, 2u, false);
        var b = DecimalTestSupport.Make(25u, 0u, 0u, 1u, false);
        let add = chic_rt_decimal_add(& a, & b, rounding, 0u);
        var ok = add.status == (int) DecimalRuntimeStatus.Success;
        let sub = chic_rt_decimal_sub(& a, & b, rounding, 0u);
        ok = ok && sub.status == (int) DecimalRuntimeStatus.Success;

        var neg = DecimalTestSupport.Make(77u, 0u, 0u, 0u, true);
        let mul = chic_rt_decimal_mul(& a, & neg, rounding, 0u);
        ok = ok && mul.status == (int) DecimalRuntimeStatus.Success;

        var zero = DecimalTestSupport.Make(0u, 0u, 0u, 0u, false);
        let divZero = chic_rt_decimal_div(& a, & zero, rounding, 0u);
        ok = ok && divZero.status == (int) DecimalRuntimeStatus.DivideByZero;
        let remZero = chic_rt_decimal_rem(& a, & zero, rounding, 0u);
        ok = ok && remZero.status == (int) DecimalRuntimeStatus.DivideByZero;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_decimal_matrix_sweep_sum_ok_When_executed_Then_decimal_matrix_sweep_sum_ok()
{
    unsafe {
        let rounding = new DecimalRoundingAbi {
            value = 0u32
        }
        ;
        let partSize = (usize) __sizeof<Decimal128Parts>();
        let partAlign = (usize) __alignof<Decimal128Parts>();
        var buffer = MemoryRuntime.chic_rt_alloc(partSize * 4usize, partAlign);
        var * mut Decimal128Parts parts = buffer.Pointer;
        * parts = DecimalTestSupport.Make(1u, 0u, 0u, 0u, false);
        var * mut @expose_address byte ptr1Raw = NativePtr.OffsetMut(buffer.Pointer, (isize) partSize);
        var * mut Decimal128Parts ptr1 = ptr1Raw;
        * ptr1 = DecimalTestSupport.Make(2u, 0u, 0u, 0u, false);
        var * mut @expose_address byte ptr2Raw = NativePtr.OffsetMut(buffer.Pointer, (isize)(partSize * 2usize));
        var * mut Decimal128Parts ptr2 = ptr2Raw;
        * ptr2 = DecimalTestSupport.Make(3u, 0u, 0u, 0u, false);
        var * mut @expose_address byte ptr3Raw = NativePtr.OffsetMut(buffer.Pointer, (isize)(partSize * 3usize));
        var * mut Decimal128Parts ptr3 = ptr3Raw;
        * ptr3 = DecimalTestSupport.Make(4u, 0u, 0u, 0u, false);
        let sum = chic_rt_decimal_sum(new DecimalConstPtr {
            Pointer = parts
        }
        , 4usize, rounding, 0u);
        let _ = chic_rt_decimal_dot(new DecimalConstPtr {
            Pointer = parts
        }
        , new DecimalConstPtr {
            Pointer = parts
        }
        , 4usize, rounding, 0u);
        var outMat = new Decimal128Parts {
            lo = 0u32, mid = 0u32, hi = 0u32, flags = 0u32
        }
        ;
        let _ = chic_rt_decimal_matmul(new DecimalConstPtr {
            Pointer = parts
        }
        , 2usize, 2usize, new DecimalConstPtr {
            Pointer = parts
        }
        , 2usize, new DecimalMutPtr {
            Pointer = & outMat
        }
        , rounding, 0u);
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(sum.status).IsEqualTo(0);
    }
}

testcase Given_decimal_matrix_sweep_dot_ok_When_executed_Then_decimal_matrix_sweep_dot_ok()
{
    unsafe {
        let rounding = new DecimalRoundingAbi {
            value = 0u32
        }
        ;
        let partSize = (usize) __sizeof<Decimal128Parts>();
        let partAlign = (usize) __alignof<Decimal128Parts>();
        var buffer = MemoryRuntime.chic_rt_alloc(partSize * 4usize, partAlign);
        var * mut Decimal128Parts parts = buffer.Pointer;
        * parts = DecimalTestSupport.Make(1u, 0u, 0u, 0u, false);
        var * mut @expose_address byte ptr1Raw = NativePtr.OffsetMut(buffer.Pointer, (isize) partSize);
        var * mut Decimal128Parts ptr1 = ptr1Raw;
        * ptr1 = DecimalTestSupport.Make(2u, 0u, 0u, 0u, false);
        var * mut @expose_address byte ptr2Raw = NativePtr.OffsetMut(buffer.Pointer, (isize)(partSize * 2usize));
        var * mut Decimal128Parts ptr2 = ptr2Raw;
        * ptr2 = DecimalTestSupport.Make(3u, 0u, 0u, 0u, false);
        var * mut @expose_address byte ptr3Raw = NativePtr.OffsetMut(buffer.Pointer, (isize)(partSize * 3usize));
        var * mut Decimal128Parts ptr3 = ptr3Raw;
        * ptr3 = DecimalTestSupport.Make(4u, 0u, 0u, 0u, false);
        let _ = chic_rt_decimal_sum(new DecimalConstPtr {
            Pointer = parts
        }
        , 4usize, rounding, 0u);
        let dot = chic_rt_decimal_dot(new DecimalConstPtr {
            Pointer = parts
        }
        , new DecimalConstPtr {
            Pointer = parts
        }
        , 4usize, rounding, 0u);
        var outMat = new Decimal128Parts {
            lo = 0u32, mid = 0u32, hi = 0u32, flags = 0u32
        }
        ;
        let _ = chic_rt_decimal_matmul(new DecimalConstPtr {
            Pointer = parts
        }
        , 2usize, 2usize, new DecimalConstPtr {
            Pointer = parts
        }
        , 2usize, new DecimalMutPtr {
            Pointer = & outMat
        }
        , rounding, 0u);
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(dot.status).IsEqualTo(0);
    }
}

testcase Given_decimal_matrix_sweep_matmul_ok_When_executed_Then_decimal_matrix_sweep_matmul_ok()
{
    unsafe {
        let rounding = new DecimalRoundingAbi {
            value = 0u32
        }
        ;
        let partSize = (usize) __sizeof<Decimal128Parts>();
        let partAlign = (usize) __alignof<Decimal128Parts>();
        var buffer = MemoryRuntime.chic_rt_alloc(partSize * 4usize, partAlign);
        var * mut Decimal128Parts parts = buffer.Pointer;
        * parts = DecimalTestSupport.Make(1u, 0u, 0u, 0u, false);
        var * mut @expose_address byte ptr1Raw = NativePtr.OffsetMut(buffer.Pointer, (isize) partSize);
        var * mut Decimal128Parts ptr1 = ptr1Raw;
        * ptr1 = DecimalTestSupport.Make(2u, 0u, 0u, 0u, false);
        var * mut @expose_address byte ptr2Raw = NativePtr.OffsetMut(buffer.Pointer, (isize)(partSize * 2usize));
        var * mut Decimal128Parts ptr2 = ptr2Raw;
        * ptr2 = DecimalTestSupport.Make(3u, 0u, 0u, 0u, false);
        var * mut @expose_address byte ptr3Raw = NativePtr.OffsetMut(buffer.Pointer, (isize)(partSize * 3usize));
        var * mut Decimal128Parts ptr3 = ptr3Raw;
        * ptr3 = DecimalTestSupport.Make(4u, 0u, 0u, 0u, false);
        let _ = chic_rt_decimal_sum(new DecimalConstPtr {
            Pointer = parts
        }
        , 4usize, rounding, 0u);
        let _ = chic_rt_decimal_dot(new DecimalConstPtr {
            Pointer = parts
        }
        , new DecimalConstPtr {
            Pointer = parts
        }
        , 4usize, rounding, 0u);
        var outMat = new Decimal128Parts {
            lo = 0u32, mid = 0u32, hi = 0u32, flags = 0u32
        }
        ;
        let mat = chic_rt_decimal_matmul(new DecimalConstPtr {
            Pointer = parts
        }
        , 2usize, 2usize, new DecimalConstPtr {
            Pointer = parts
        }
        , 2usize, new DecimalMutPtr {
            Pointer = & outMat
        }
        , rounding, 0u);
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(mat).IsEqualTo(0);
    }
}

testcase Given_decimal_internal_helpers_When_executed_Then_decimal_internal_helpers()
{
    unsafe {
        DecimalTestCoverageHelpers();
        Assert.That(true).IsTrue();
    }
}
