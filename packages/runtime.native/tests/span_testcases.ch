namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import static Std.Runtime.Native.SpanRuntime;
import Std.Runtime.Native.Testing;

private unsafe static bool SpanBytesEqual(* const @readonly @expose_address byte left,
* const @readonly @expose_address byte right, usize len) {
    var idx = 0usize;
    while (idx < len)
    {
        let leftPtr = NativePtr.OffsetConst(left, (isize) idx);
        let rightPtr = NativePtr.OffsetConst(right, (isize) idx);
        let leftValue = NativePtr.ReadByteConst(leftPtr);
        let rightValue = NativePtr.ReadByteConst(rightPtr);
        if (leftValue != rightValue)
        {
            return false;
        }
        idx += 1usize;
    }
    return true;
}

testcase Given_span_slice_and_ptr_access_When_executed_Then_span_slice_and_ptr_access()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(4usize, 1usize);
        var ok = !NativePtr.IsNull(buffer.Pointer);
        var idx = 0usize;
        while (idx < 4usize)
        {
            let ptr = NativePtr.OffsetMut(buffer.Pointer, (isize) idx);
            * ptr = (byte)(idx + 1usize);
            idx = idx + 1;
        }
        var handle = new ValueMutPtr {
            Pointer = buffer.Pointer, Size = 1usize, Alignment = 1usize,
        }
        ;
        var span = chic_rt_span_from_raw_mut(& handle, 4usize);
        var slice = span;
        let status = chic_rt_span_slice_mut(& span, 1usize, 2usize, & slice);
        ok = ok && status == 0;
        var dst = MemoryRuntime.chic_rt_alloc(2usize, 1usize);
        var dstHandle = new ValueMutPtr {
            Pointer = dst.Pointer, Size = 1usize, Alignment = 1usize,
        }
        ;
        var destSpan = chic_rt_span_from_raw_mut(& dstHandle, 2usize);
        let readonlySlice = chic_rt_span_to_readonly(& slice);
        let copyStatus = chic_rt_span_copy_to(& readonlySlice, & destSpan);
        ok = ok && copyStatus == 0;
        let first = NativePtr.ReadByteMut(dst.Pointer);
        let second = NativePtr.ReadByteMut(NativePtr.OffsetMut(dst.Pointer, 1));
        ok = ok && first == 2u8;
        ok = ok && second == 3u8;
        MemoryRuntime.chic_rt_free(dst);
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_span_copy_and_fill_When_executed_Then_span_copy_and_fill()
{
    unsafe {
        var srcBlock = MemoryRuntime.chic_rt_alloc(3usize, 1usize);
        var dstBlock = MemoryRuntime.chic_rt_alloc(3usize, 1usize);
        var idx = 0usize;
        while (idx < 3usize)
        {
            let ptr = NativePtr.OffsetMut(srcBlock.Pointer, (isize) idx);
            * ptr = (byte)(10usize + idx);
            idx = idx + 1;
        }
        var srcHandle = new ValueConstPtr {
            Pointer = NativePtr.AsConstPtr(srcBlock.Pointer), Size = 1usize, Alignment = 1usize,
        }
        ;
        var dstHandle = new ValueMutPtr {
            Pointer = dstBlock.Pointer, Size = 1usize, Alignment = 1usize,
        }
        ;
        var roSpan = chic_rt_span_from_raw_const(& srcHandle, 3usize);
        var mutSpan = chic_rt_span_from_raw_mut(& dstHandle, 3usize);
        let copyStatus = chic_rt_span_copy_to(& roSpan, & mutSpan);
        var ok = copyStatus == 0;
        ok = ok && SpanBytesEqual(srcBlock.Pointer, dstBlock.Pointer, 3usize);
        let fillValue = 0x7Fu8;
        let fillStatus = chic_rt_span_fill(& mutSpan, & fillValue);
        ok = ok && fillStatus == 0;
        idx = 0usize;
        var fillOk = true;
        while (idx < 3usize)
        {
            let ptr = NativePtr.OffsetMut(dstBlock.Pointer, (isize) idx);
            let value = NativePtr.ReadByteMut(ptr);
            if (value != fillValue)
            {
                fillOk = false;
            }
            idx = idx + 1;
        }
        MemoryRuntime.chic_rt_free(srcBlock);
        MemoryRuntime.chic_rt_free(dstBlock);
        ok = ok && fillOk;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_span_layout_and_error_paths_When_executed_Then_span_layout_and_error_paths()
{
    unsafe {
        var info = new SpanLayoutInfo {
            size = 0, offset_data = 0, offset_reserved = 0, offset_len = 0, offset_elem_size = 0, offset_elem_align = 0,
        }
        ;
        chic_rt_span_layout_debug(& info);
        var ok = info.size >0usize;
        ok = ok && info.offset_elem_align >0usize;

        var nullHandle = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 4usize, Alignment = 4usize
        }
        ;
        var nullSpan = chic_rt_span_from_raw_mut(& nullHandle, 2usize);
        let nullDest = chic_rt_span_slice_mut(& nullSpan, 0usize, 1usize, (* mut ChicSpan) NativePtr.NullMut());
        ok = ok && nullDest == 1;
        let outOfBounds = chic_rt_span_slice_mut(& nullSpan, 3usize, 1usize, & nullSpan);
        ok = ok && outOfBounds == 2;

        var block = MemoryRuntime.chic_rt_alloc(8usize, 1usize);
        var misaligned = NativePtr.OffsetMut(block.Pointer, 1isize);
        var badHandle = new ValueMutPtr {
            Pointer = misaligned, Size = 4usize, Alignment = 4usize
        }
        ;
        var badSpan = chic_rt_span_from_raw_mut(& badHandle, 1usize);
        var outSpan = badSpan;
        let invalidStride = chic_rt_span_slice_mut(& badSpan, 0usize, 1usize, & outSpan);
        ok = ok && invalidStride == 3;

        var goodHandle = new ValueMutPtr {
            Pointer = block.Pointer, Size = 1usize, Alignment = 1usize
        }
        ;
        var goodSpan = chic_rt_span_from_raw_mut(& goodHandle, 4usize);
        let okPtr = chic_rt_span_ptr_at_mut(& goodSpan, 2usize);
        ok = ok && !NativePtr.IsNull(okPtr);
        let oobPtr = chic_rt_span_ptr_at_mut(& goodSpan, 7usize);
        ok = ok && NativePtr.IsNull(oobPtr);

        var roHandle = new ValueConstPtr {
            Pointer = NativePtr.AsConstPtr(block.Pointer), Size = 1usize, Alignment = 1usize
        }
        ;
        var roSpan = chic_rt_span_from_raw_const(& roHandle, 2usize);
        let roPtr = chic_rt_span_ptr_at_readonly(& roSpan, 0usize);
        ok = ok && !NativePtr.IsNullConst(roPtr);
        let roOob = chic_rt_span_ptr_at_readonly(& roSpan, 5usize);
        ok = ok && NativePtr.IsNullConst(roOob);
        MemoryRuntime.chic_rt_free(block);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_span_copy_and_fill_error_paths_When_executed_Then_span_copy_and_fill_error_paths()
{
    unsafe {
        var block = MemoryRuntime.chic_rt_alloc(4usize, 1usize);
        var srcHandle = new ValueConstPtr {
            Pointer = NativePtr.AsConstPtr(block.Pointer), Size = 1usize, Alignment = 1usize
        }
        ;
        var dstHandle = new ValueMutPtr {
            Pointer = block.Pointer, Size = 2usize, Alignment = 1usize
        }
        ;
        var srcSpan = chic_rt_span_from_raw_const(& srcHandle, 4usize);
        var dstSpan = chic_rt_span_from_raw_mut(& dstHandle, 2usize);
        let tooShort = chic_rt_span_copy_to(& srcSpan, & dstSpan);
        var ok = tooShort == 2;

        var badHandle = new ValueMutPtr {
            Pointer = block.Pointer, Size = 2usize, Alignment = 1usize
        }
        ;
        var badSpan = chic_rt_span_from_raw_mut(& badHandle, 4usize);
        let badStride = chic_rt_span_copy_to(& srcSpan, & badSpan);
        ok = ok && badStride == 3;

        let nullFill = chic_rt_span_fill(& badSpan, NativePtr.NullConst());
        ok = ok && nullFill == 1;
        MemoryRuntime.chic_rt_free(block);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_span_zero_len_and_invalid_stride_When_executed_Then_span_zero_len_and_invalid_stride()
{
    unsafe {
        var badHandle = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 1usize, Alignment = 3usize
        }
        ;
        var badSpan = chic_rt_span_from_raw_mut(& badHandle, 0usize);
        var outSpan = badSpan;
        let invalidStride = chic_rt_span_slice_mut(& badSpan, 0usize, 0usize, & outSpan);
        var ok = invalidStride == 3;

        var emptyHandle = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 1usize, Alignment = 1usize
        }
        ;
        var emptySpan = chic_rt_span_from_raw_mut(& emptyHandle, 0usize);
        let fillValue = 1u8;
        let fillStatus = chic_rt_span_fill(& emptySpan, & fillValue);
        ok = ok && fillStatus == 0;

        let readonlySpan = chic_rt_span_to_readonly((* const ChicSpan) NativePtr.NullConst());
        ok = ok && readonlySpan.len == 0usize;

        var zeroHandle = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        var zeroSpan = chic_rt_span_from_raw_mut(& zeroHandle, 3usize);
        let ptr = chic_rt_span_ptr_at_mut(& zeroSpan, 2usize);
        ok = ok && !NativePtr.IsNull(ptr);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_span_slice_readonly_and_bounds_When_executed_Then_span_slice_readonly_and_bounds()
{
    unsafe {
        var block = MemoryRuntime.chic_rt_alloc(5usize, 1usize);
        var idx = 0usize;
        while (idx < 5usize)
        {
            let ptr = NativePtr.OffsetMut(block.Pointer, (isize) idx);
            * ptr = (byte)(idx + 1usize);
            idx = idx + 1usize;
        }
        var roHandle = new ValueConstPtr {
            Pointer = NativePtr.AsConstPtr(block.Pointer), Size = 1usize, Alignment = 1usize
        }
        ;
        var roSpan = chic_rt_span_from_raw_const(& roHandle, 5usize);
        var slice = roSpan;
        let ok = chic_rt_span_slice_readonly(& roSpan, 1usize, 3usize, & slice);
        var success = ok == 0;
        success = success && slice.len == 3usize;
        let oob = chic_rt_span_slice_readonly(& roSpan, 4usize, 2usize, & slice);
        success = success && oob == 2;
        MemoryRuntime.chic_rt_free(block);
        Assert.That(success).IsTrue();
    }
}

testcase Given_span_layout_debug_and_null_slices_When_executed_Then_span_layout_debug_and_null_slices()
{
    unsafe {
        var info = new SpanLayoutInfo {
            size = 0usize, offset_data = 0usize, offset_reserved = 0usize, offset_len = 0usize, offset_elem_size = 0usize,
            offset_elem_align = 0usize
        }
        ;
        chic_rt_span_layout_debug(& info);
        var ok = info.size > 0usize;

        var outSpan = new ChicSpan {
            data = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
            }
            , len = 0usize, elem_size = 0usize, elem_align = 1usize,
        }
        ;
        let nullStatus = chic_rt_span_slice_mut((* const ChicSpan) NativePtr.NullConst(), 0usize, 0usize, & outSpan);
        ok = ok && nullStatus == 1;
        let nullRoStatus = chic_rt_span_slice_readonly((* const ChicReadOnlySpan) NativePtr.NullConst(), 0usize, 0usize,
        (* mut ChicReadOnlySpan) NativePtr.NullMut());
        ok = ok && nullRoStatus == 1;

        let nullPtr = chic_rt_span_ptr_at_readonly((* const ChicReadOnlySpan) NativePtr.NullConst(), 0usize);
        ok = ok && nullPtr == null;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_span_readonly_slice_and_copy_When_executed_Then_span_readonly_slice_and_copy()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(6usize, 1usize);
        var idx = 0usize;
        while (idx < 6usize)
        {
            let ptr = NativePtr.OffsetMut(buffer.Pointer, (isize) idx);
            * ptr = (byte)(20usize + idx);
            idx = idx + 1;
        }
        var srcHandle = new ValueConstPtr {
            Pointer = NativePtr.AsConstPtr(buffer.Pointer), Size = 1usize, Alignment = 1usize
        }
        ;
        var roSpan = chic_rt_span_from_raw_const(& srcHandle, 6usize);
        var sliced = roSpan;
        let sliceStatus = chic_rt_span_slice_readonly(& roSpan, 2usize, 3usize, & sliced);
        var ok = sliceStatus == 0;

        var dest = MemoryRuntime.chic_rt_alloc(3usize, 1usize);
        var destHandle = new ValueMutPtr {
            Pointer = dest.Pointer, Size = 1usize, Alignment = 1usize
        }
        ;
        var destSpan = chic_rt_span_from_raw_mut(& destHandle, 3usize);
        let copyStatus = chic_rt_span_copy_to(& sliced, & destSpan);
        ok = ok && copyStatus == 0;
        ok = ok && SpanBytesEqual(NativePtr.OffsetConst(buffer.Pointer, 2isize), NativePtr.AsConstPtr(dest.Pointer), 3usize);

        let invalid = chic_rt_span_slice_readonly(& roSpan, 10usize, 1usize, & sliced);
        ok = ok && invalid == 2;
        MemoryRuntime.chic_rt_free(buffer);
        MemoryRuntime.chic_rt_free(dest);
        Assert.That(ok).IsTrue();
    }
}
