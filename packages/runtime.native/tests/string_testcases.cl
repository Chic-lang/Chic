namespace Std.Runtime.Native;
private unsafe static bool BytesEqual(* const @readonly @expose_address byte left, * const @readonly @expose_address byte right,
usize len) {
    var idx = 0usize;
    while (idx <len)
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
testcase Given_boolean_and_assignment_chain_When_executed_Then_returns_true()
{
    var ok = true;
    ok = ok && true;
    ok = ok && true;
    return ok;
}
testcase Given_logical_and_with_equality_When_executed_Then_returns_true()
{
    let v = 84u8;
    var ok = true;
    ok = ok && v == 84u8;
    ok = ok && 4usize == 4usize;
    ok = ok && 0i32 == 0i32;
    return ok;
}
testcase Given_logical_and_with_byte_load_and_equality_When_executed_Then_returns_true()
{
    unsafe {
        var tmp = new StringInlineBytes32 {
            b00 = 84, b01 = 0,
        }
        ;
        let ptr = NativePtr.AsConstPtr(& tmp.b00);
        var ok = true;
        ok = ok && NativePtr.ReadByteConst(ptr) == 84u8;
        return ok;
    }
}
testcase Given_string_push_and_append_primitives_When_executed_Then_string_push_and_append_primitives()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(2usize, 1usize);
        let hPtr = NativePtr.OffsetMut(buffer.Pointer, 0isize);
        let iPtr = NativePtr.OffsetMut(buffer.Pointer, 1isize);
        * hPtr = 104u8;
        * iPtr = 105u8;
        var slice = new ChicStr {
            ptr = NativePtr.AsConstPtr(buffer.Pointer), len = 2usize
        }
        ;
        var str = StringRuntime.chic_rt_string_new();
        if (StringRuntime.chic_rt_string_push_slice (& str, slice) != 0)
        {
            StringRuntime.chic_rt_string_drop(& str);
            MemoryRuntime.chic_rt_free(buffer);
            return false;
        }
        if (StringRuntime.chic_rt_string_append_bool (& str, true, 0, 0, new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        ) != 0) {
            StringRuntime.chic_rt_string_drop(& str);
            MemoryRuntime.chic_rt_free(buffer);
            return false;
        }
        if (StringRuntime.chic_rt_string_append_char (& str, 33u32, 0, 0, new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        ) != 0) {
            StringRuntime.chic_rt_string_drop(& str);
            MemoryRuntime.chic_rt_free(buffer);
            return false;
        }
        if (StringRuntime.chic_rt_string_append_signed (& str, 12, 0, 32u32, 0, 0, new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        ) != 0) {
            StringRuntime.chic_rt_string_drop(& str);
            MemoryRuntime.chic_rt_free(buffer);
            return false;
        }
        if (StringRuntime.chic_rt_string_append_unsigned (& str, 15u64, 0u64, 32u32, 0, 0, new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        ) != 0) {
            StringRuntime.chic_rt_string_drop(& str);
            MemoryRuntime.chic_rt_free(buffer);
            return false;
        }
        if (StringRuntime.chic_rt_string_append_f16 (& str, 0u16, 0, 0, new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        ) != 0) {
            StringRuntime.chic_rt_string_drop(& str);
            MemoryRuntime.chic_rt_free(buffer);
            return false;
        }
        if (StringRuntime.chic_rt_string_append_f32 (& str, 1.25f, 0, 0, new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        ) != 0) {
            StringRuntime.chic_rt_string_drop(& str);
            MemoryRuntime.chic_rt_free(buffer);
            return false;
        }
        if (StringRuntime.chic_rt_string_append_f64 (& str, 2.5d, 0, 0, new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        ) != 0) {
            StringRuntime.chic_rt_string_drop(& str);
            MemoryRuntime.chic_rt_free(buffer);
            return false;
        }
        if (StringRuntime.chic_rt_string_append_f128 (& str, 0u128, 0, 0, new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        ) != 0) {
            StringRuntime.chic_rt_string_drop(& str);
            MemoryRuntime.chic_rt_free(buffer);
            return false;
        }
        let outSlice = StringRuntime.chic_rt_string_as_slice(& str);
        if (outSlice.len <2usize)
        {
            StringRuntime.chic_rt_string_drop(& str);
            MemoryRuntime.chic_rt_free(buffer);
            return false;
        }
        let outH = NativePtr.ReadByteConst(outSlice.ptr);
        let outI = NativePtr.ReadByteConst(NativePtr.OffsetConst(outSlice.ptr, 1isize));
        if (outH != 104u8)
        {
            StringRuntime.chic_rt_string_drop(& str);
            MemoryRuntime.chic_rt_free(buffer);
            return false;
        }
        if (outI != 105u8)
        {
            StringRuntime.chic_rt_string_drop(& str);
            MemoryRuntime.chic_rt_free(buffer);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& str);
        MemoryRuntime.chic_rt_free(buffer);
        return true;
    }
}
testcase Given_string_append_f128_nan_When_executed_Then_prefix_is_nan()
{
    unsafe {
        var str = StringRuntime.chic_rt_string_new();
        let bits = (0x7FFFu128 << 112) | 1u128;
        if (StringRuntime.chic_rt_string_append_f128 (& str, bits, 0, 0, new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        ) != 0) {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        let slice = StringRuntime.chic_rt_string_as_slice(& str);
        if (slice.len <3usize)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        if (NativePtr.ReadByteConst (slice.ptr) != 110u8)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        if (NativePtr.ReadByteConst (NativePtr.OffsetConst (slice.ptr, 1isize)) != 97u8)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        if (NativePtr.ReadByteConst (NativePtr.OffsetConst (slice.ptr, 2isize)) != 110u8)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& str);
        return true;
    }
}
testcase Given_string_clone_truncate_and_errors_When_executed_Then_string_clone_truncate_and_errors()
{
    unsafe {
        var str = StringRuntime.chic_rt_string_from_char(65u32);
        var clone = StringRuntime.chic_rt_string_new();
        if (StringRuntime.chic_rt_string_clone (& clone, & str) != 0)
        {
            StringRuntime.chic_rt_string_drop(& clone);
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        let before = StringRuntime.chic_rt_string_as_slice(& clone);
        if (before.len != 1usize)
        {
            StringRuntime.chic_rt_string_drop(& clone);
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        if (StringRuntime.chic_rt_string_truncate (& clone, 0usize) != 0)
        {
            StringRuntime.chic_rt_string_drop(& clone);
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        if (StringRuntime.chic_rt_string_truncate (& clone, 2usize) != 5)
        {
            StringRuntime.chic_rt_string_drop(& clone);
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        if (StringRuntime.chic_rt_string_reserve (& clone, 0usize) != 0)
        {
            StringRuntime.chic_rt_string_drop(& clone);
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        let message = StringRuntime.chic_rt_string_error_message(4);
        if (message.len == 0usize)
        {
            StringRuntime.chic_rt_string_drop(& clone);
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        NativeAlloc.Free(new ValueMutPtr {
            Pointer = NativePtr.AsMutPtr(message.ptr), Size = message.len, Alignment = 1usize
        }
        );
        if (StringRuntime.chic_rt_string_debug_ping () != 42)
        {
            StringRuntime.chic_rt_string_drop(& clone);
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& clone);
        StringRuntime.chic_rt_string_drop(& str);
        return true;
    }
}
testcase Given_string_accessors_and_slices_When_executed_Then_string_accessors_and_slices()
{
    unsafe {
        var data = new StringInlineBytes64 {
            b00 = 97, b01 = 98, b02 = 99,
        }
        ;
        var slice = new ChicStr {
            ptr = NativePtr.AsConstPtr(& data.b00), len = 3usize
        }
        ;
        var str = StringRuntime.chic_rt_string_with_capacity(8usize);
        var ok = StringRuntime.chic_rt_string_append_slice(& str, slice, 0, 0) == 0;
        ok = ok && StringRuntime.chic_rt_string_get_len(& str) == 3usize;
        let cap = StringRuntime.chic_rt_string_get_cap(& str);
        ok = ok && cap >= 8usize;
        StringRuntime.chic_rt_string_set_len(& str, 2usize);
        ok = ok && StringRuntime.chic_rt_string_get_len(& str) == 2usize;
        StringRuntime.chic_rt_string_set_len(& str, 3usize);
        let ptr = StringRuntime.chic_rt_string_get_ptr(& str);
        ok = ok && !NativePtr.IsNull(ptr);
        StringRuntime.chic_rt_string_set_ptr(& str, ptr);
        StringRuntime.chic_rt_string_set_cap(& str, cap);
        let inlineCap = StringRuntime.chic_rt_string_inline_capacity();
        ok = ok && inlineCap >0usize;
        let inlinePtr = StringRuntime.chic_rt_string_inline_ptr(& str);
        ok = ok && !NativePtr.IsNull(inlinePtr);
        let chars = StringRuntime.chic_rt_string_as_chars(& str);
        ok = ok && chars.len == 3usize;
        let strChars = StringRuntime.chic_rt_str_as_chars(StringRuntime.chic_rt_string_as_slice(& str));
        ok = ok && strChars.len == 3usize;
        StringRuntime.chic_rt_string_drop(& str);
        return ok;
    }
}
testcase Given_string_error_paths_cover_invalid_inputs_When_executed_Then_string_error_paths_cover_invalid_inputs()
{
    unsafe {
        var str = StringRuntime.chic_rt_string_new();
        let badSlice = new ChicStr {
            ptr = NativePtr.NullConst(), len = 2usize
        }
        ;
        var ok = StringRuntime.chic_rt_string_push_slice(& str, badSlice) == 4;
        let cloneStatus = StringRuntime.chic_rt_string_clone_slice((* mut ChicString) NativePtr.NullMut(), badSlice);
        ok = ok && cloneStatus == 4;
        let badChar = StringRuntime.chic_rt_string_append_char(& str, 0x110000u32, 0, 0, new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        );
        ok = ok && badChar == 1;
        StringRuntime.chic_rt_string_drop(& str);
        return ok;
    }
}
testcase Given_string_from_slice_and_char_encoding_When_executed_Then_string_from_slice_and_char_encoding()
{
    unsafe {
        var data = new StringInlineBytes32 {
            b00 = 104, b01 = 101, b02 = 121,
        }
        ;
        var slice = new ChicStr {
            ptr = NativePtr.AsConstPtr(& data.b00), len = 3usize
        }
        ;
        var str = StringRuntime.chic_rt_string_from_slice(slice);
        let sliceOut = StringRuntime.chic_rt_string_as_slice(& str);
        var ok = sliceOut.len == 3usize;
        ok = ok && BytesEqual(sliceOut.ptr, slice.ptr, 3usize);
        ok = ok && StringRuntime.FirstByteViaByValue(str) == 104u8;
        StringRuntime.chic_rt_string_drop(& str);
        var str2 = StringRuntime.chic_rt_string_from_char(0x1F600u32);
        let out2 = StringRuntime.chic_rt_string_as_slice(& str2);
        ok = ok && out2.len == 4usize;
        StringRuntime.chic_rt_string_drop(& str2);
        return ok;
    }
}
testcase Given_string_numeric_formatting_variants_When_executed_Then_string_numeric_formatting_variants()
{
    unsafe {
        var fmtHex = new StringInlineBytes32 {
            b00 = 88, b01 = 48, b02 = 52,
        }
        ;
        var hexFmt = new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtHex.b00), len = 3usize
        }
        ;
        var str = StringRuntime.chic_rt_string_new();
        let statusHex = StringRuntime.chic_rt_string_append_unsigned(& str, 0xABu64, 0u64, 8u32, 0, 0, hexFmt);
        if (statusHex != 0)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        let outHex = StringRuntime.chic_rt_string_as_slice(& str);
        var expectedHex = new StringInlineBytes32 {
            b00 = 48, b01 = 48, b02 = 65, b03 = 66,
        }
        ;
        if (outHex.len != 4usize)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        if (!BytesEqual (outHex.ptr, NativePtr.AsConstPtr (& expectedHex.b00), 4usize))
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& str);
        var fmtLower = new StringInlineBytes32 {
            b00 = 120, b01 = 50,
        }
        ;
        var lowerFmt = new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtLower.b00), len = 2usize
        }
        ;
        var str2 = StringRuntime.chic_rt_string_new();
        let statusLower = StringRuntime.chic_rt_string_append_unsigned(& str2, 0x1Fu64, 0u64, 8u32, 0, 0, lowerFmt);
        if (statusLower != 0)
        {
            StringRuntime.chic_rt_string_drop(& str2);
            return false;
        }
        let outLower = StringRuntime.chic_rt_string_as_slice(& str2);
        var expectedLower = new StringInlineBytes32 {
            b00 = 49, b01 = 102,
        }
        ;
        if (outLower.len != 2usize)
        {
            StringRuntime.chic_rt_string_drop(& str2);
            return false;
        }
        if (!BytesEqual (outLower.ptr, NativePtr.AsConstPtr (& expectedLower.b00), 2usize))
        {
            StringRuntime.chic_rt_string_drop(& str2);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& str2);
        var str3 = StringRuntime.chic_rt_string_new();
        let statusSigned = StringRuntime.chic_rt_string_append_signed(& str3, - 42, - 1, 64u32, 5, 1, new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        );
        if (statusSigned != 0)
        {
            StringRuntime.chic_rt_string_drop(& str3);
            return false;
        }
        let outSigned = StringRuntime.chic_rt_string_as_slice(& str3);
        if (outSigned.len <3usize)
        {
            StringRuntime.chic_rt_string_drop(& str3);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& str3);
        return true;
    }
}
testcase Given_string_append_unsigned_high_word_When_executed_Then_formats_high_word()
{
    unsafe {
        var fmtHex = new StringInlineBytes32 {
            b00 = 88, b01 = 0,
        }
        ;
        let fmtSlice = new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtHex.b00), len = 1usize
        }
        ;
        var str = StringRuntime.chic_rt_string_new();
        let status = StringRuntime.chic_rt_string_append_unsigned(& str, 0u64, 1u64, 128u32, 0, 0, fmtSlice);
        let outSlice = StringRuntime.chic_rt_string_as_slice(& str);
        if (status != 0)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        if (outSlice.len != 17usize)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        if (NativePtr.ReadByteConst (outSlice.ptr) != 49u8)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        if (NativePtr.ReadByteConst (NativePtr.OffsetConst (outSlice.ptr, 16isize)) != 48u8)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& str);
        return true;
    }
}
testcase Given_string_format_sweep_When_executed_Then_string_format_sweep()
{
    unsafe {
        var ok = true;
        var str = StringRuntime.chic_rt_string_new();
        var fmtHex = new StringInlineBytes32 {
            b00 = 88, b01 = 52, b02 = 0,
        }
        ;
        var fmtHexStr = new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtHex.b00), len = 2usize
        }
        ;
        ok = ok && StringRuntime.chic_rt_string_append_unsigned(& str, 0xFu64, 0u64, 32u32, 0, 0, fmtHexStr) == 0;
        var fmtLower = new StringInlineBytes32 {
            b00 = 120, b01 = 49, b02 = 0,
        }
        ;
        var fmtLowerStr = new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtLower.b00), len = 2usize
        }
        ;
        ok = ok && StringRuntime.chic_rt_string_append_unsigned(& str, 0xAu64, 0u64, 16u32, 4, 1, fmtLowerStr) == 0;
        var fmtFloat = new StringInlineBytes32 {
            b00 = 70, b01 = 51, b02 = 0,
        }
        ;
        var fmtFloatStr = new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtFloat.b00), len = 2usize
        }
        ;
        ok = ok && StringRuntime.chic_rt_string_append_f64(& str, 123.456, 0, 0, fmtFloatStr) == 0;
        var fmtExp = new StringInlineBytes32 {
            b00 = 69, b01 = 49, b02 = 0,
        }
        ;
        var fmtExpStr = new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtExp.b00), len = 2usize
        }
        ;
        ok = ok && StringRuntime.chic_rt_string_append_f64(& str, 0.001, 0, 0, fmtExpStr) == 0;
        var fmtGeneral = new StringInlineBytes32 {
            b00 = 71, b01 = 50, b02 = 0,
        }
        ;
        var fmtGeneralStr = new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtGeneral.b00), len = 2usize
        }
        ;
        ok = ok && StringRuntime.chic_rt_string_append_f64(& str, 1.0, 0, 0, fmtGeneralStr) == 0;
        var fmtBad = new StringInlineBytes32 {
            b00 = 88, b01 = 65, b02 = 0,
        }
        ;
        var fmtBadStr = new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtBad.b00), len = 2usize
        }
        ;
        let badStatus = StringRuntime.chic_rt_string_append_unsigned(& str, 1u64, 0u64, 32u32, 0, 0, fmtBadStr);
        ok = ok && badStatus == (int) StringError.InvalidPointer;
        StringRuntime.chic_rt_string_drop(& str);
        return ok;
    }
}
testcase Given_string_float_formats_and_specials_When_executed_Then_string_float_formats_and_specials()
{
    unsafe {
        var fmtE = new StringInlineBytes32 {
            b00 = 69, b01 = 50, b02 = 0,
        }
        ;
        var fmtG = new StringInlineBytes32 {
            b00 = 103, b01 = 51, b02 = 0,
        }
        ;
        var str = StringRuntime.chic_rt_string_new();
        let statusE = StringRuntime.chic_rt_string_append_f64(& str, 12.5, 0, 0, new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtE.b00), len = 2usize
        }
        );
        var ok = statusE == 0;
        let outE = StringRuntime.chic_rt_string_as_slice(& str);
        ok = ok && outE.len >= 4usize;
        StringRuntime.chic_rt_string_drop(& str);
        var str2 = StringRuntime.chic_rt_string_new();
        let statusG = StringRuntime.chic_rt_string_append_f64(& str2, 0.00005, 0, 0, new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtG.b00), len = 2usize
        }
        );
        ok = ok && statusG == 0;
        let outG = StringRuntime.chic_rt_string_as_slice(& str2);
        ok = ok && outG.len >= 4usize;
        StringRuntime.chic_rt_string_drop(& str2);
        var str3 = StringRuntime.chic_rt_string_new();
        let _ = StringRuntime.chic_rt_string_append_f64(& str3, 0.0 / 0.0, 0, 0, new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        );
        let outNan = StringRuntime.chic_rt_string_as_slice(& str3);
        ok = ok && outNan.len == 3usize;
        StringRuntime.chic_rt_string_drop(& str3);
        var str4 = StringRuntime.chic_rt_string_new();
        let _ = StringRuntime.chic_rt_string_append_f64(& str4, 1.0 / 0.0, 0, 0, new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        );
        let outInf = StringRuntime.chic_rt_string_as_slice(& str4);
        ok = ok && outInf.len == 3usize;
        StringRuntime.chic_rt_string_drop(& str4);
        var str5 = StringRuntime.chic_rt_string_new();
        let _ = StringRuntime.chic_rt_string_append_f64(& str5, - 1.0 / 0.0, 0, 0, new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        );
        let outNegInf = StringRuntime.chic_rt_string_as_slice(& str5);
        ok = ok && outNegInf.len == 4usize;
        StringRuntime.chic_rt_string_drop(& str5);
        return ok;
    }
}
testcase Given_string_bool_format_variants_When_executed_Then_string_bool_format_variants()
{
    unsafe {
        var fmtUpper = new StringInlineBytes32 {
            b00 = 85,
        }
        ;
        var fmtLower = new StringInlineBytes32 {
            b00 = 108,
        }
        ;
        var str = StringRuntime.chic_rt_string_new();
        let statusUpper = StringRuntime.chic_rt_string_append_bool(& str, true, 0, 0, new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtUpper.b00), len = 1usize
        }
        );
        if (statusUpper != 0)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        let outUpper = StringRuntime.chic_rt_string_as_slice(& str);
        if (outUpper.len != 4usize)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        if (NativePtr.ReadByteConst (outUpper.ptr) != 84u8)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& str);
        var str2 = StringRuntime.chic_rt_string_new();
        let statusLower = StringRuntime.chic_rt_string_append_bool(& str2, false, 0, 0, new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtLower.b00), len = 1usize
        }
        );
        if (statusLower != 0)
        {
            StringRuntime.chic_rt_string_drop(& str2);
            return false;
        }
        let outLower = StringRuntime.chic_rt_string_as_slice(& str2);
        if (outLower.len != 5usize)
        {
            StringRuntime.chic_rt_string_drop(& str2);
            return false;
        }
        if (NativePtr.ReadByteConst (outLower.ptr) != 102u8)
        {
            StringRuntime.chic_rt_string_drop(& str2);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& str2);
        return true;
    }
}
testcase Given_string_alignment_and_format_errors_When_executed_Then_string_alignment_and_format_errors()
{
    unsafe {
        var fmtBad = new StringInlineBytes32 {
            b00 = 88, b01 = 71,
        }
        ;
        var str = StringRuntime.chic_rt_string_new();
        let badStatus = StringRuntime.chic_rt_string_append_unsigned(& str, 1u64, 0u64, 32u32, 0, 0, new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtBad.b00), len = 2usize
        }
        );
        if (badStatus != 4)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& str);
        var fmtHex = new StringInlineBytes32 {
            b00 = 88, b01 = 52,
        }
        ;
        var str2 = StringRuntime.chic_rt_string_new();
        let hexStatus = StringRuntime.chic_rt_string_append_signed(& str2, 0x1A, 0, 64u32, 6, 1, new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtHex.b00), len = 2usize
        }
        );
        if (hexStatus != 0)
        {
            StringRuntime.chic_rt_string_drop(& str2);
            return false;
        }
        let out2 = StringRuntime.chic_rt_string_as_slice(& str2);
        if (out2.len != 6usize)
        {
            StringRuntime.chic_rt_string_drop(& str2);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& str2);
        var fmtWide = new StringInlineBytes32 {
            b00 = 120, b01 = 54,
        }
        ;
        var str3 = StringRuntime.chic_rt_string_new();
        let padStatus = StringRuntime.chic_rt_string_append_unsigned(& str3, 0xBu64, 0u64, 16u32, - 6, 1, new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtWide.b00), len = 2usize
        }
        );
        if (padStatus != 0)
        {
            StringRuntime.chic_rt_string_drop(& str3);
            return false;
        }
        let out3 = StringRuntime.chic_rt_string_as_slice(& str3);
        if (out3.len != 6usize)
        {
            StringRuntime.chic_rt_string_drop(& str3);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& str3);
        return true;
    }
}
testcase Given_string_float_fixed_and_negative_zero_When_executed_Then_string_float_fixed_and_negative_zero()
{
    unsafe {
        var fmtFixed = new StringInlineBytes32 {
            b00 = 70, b01 = 50,
        }
        ;
        var str = StringRuntime.chic_rt_string_new();
        let fixedStatus = StringRuntime.chic_rt_string_append_f64(& str, 12.34, 0, 0, new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtFixed.b00), len = 2usize
        }
        );
        if (fixedStatus != 0)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        let outSlice = StringRuntime.chic_rt_string_as_slice(& str);
        if (outSlice.len <4usize)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& str);
        var fmtFixed2 = new StringInlineBytes32 {
            b00 = 70, b01 = 49,
        }
        ;
        var str2 = StringRuntime.chic_rt_string_new();
        let zeroStatus = StringRuntime.chic_rt_string_append_f64(& str2, - 0.0, 0, 0, new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtFixed2.b00), len = 2usize
        }
        );
        if (zeroStatus != 0)
        {
            StringRuntime.chic_rt_string_drop(& str2);
            return false;
        }
        let out2 = StringRuntime.chic_rt_string_as_slice(& str2);
        if (out2.len == 0usize)
        {
            StringRuntime.chic_rt_string_drop(& str2);
            return false;
        }
        if (NativePtr.ReadByteConst (out2.ptr) != 45u8)
        {
            StringRuntime.chic_rt_string_drop(& str2);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& str2);
        return true;
    }
}
testcase Given_string_reserve_and_allocation_counters_When_executed_Then_string_reserve_and_allocation_counters()
{
    unsafe {
        let beforeAlloc = StringRuntime.chic_rt_string_allocations();
        let beforeFree = StringRuntime.chic_rt_string_frees();
        var str = StringRuntime.chic_rt_string_with_capacity(0usize);
        let inlineCap = StringRuntime.chic_rt_string_inline_capacity();
        var ok = StringRuntime.chic_rt_string_reserve(& str, inlineCap + 8usize) == 0;
        let afterAlloc = StringRuntime.chic_rt_string_allocations();
        ok = ok && afterAlloc >= beforeAlloc;
        StringRuntime.chic_rt_string_drop(& str);
        let afterFree = StringRuntime.chic_rt_string_frees();
        ok = ok && afterFree >= beforeFree;
        return ok;
    }
}
testcase Given_string_large_append_and_truncate_When_executed_Then_string_large_append_and_truncate()
{
    unsafe {
        let inlineCap = StringRuntime.chic_rt_string_inline_capacity();
        let total = inlineCap + 12usize;
        var buffer = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = total, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(total, 1usize, out buffer);
        if (status != NativeAllocationError.Success)
        {
            return false;
        }
        var idx = 0usize;
        while (idx <total)
        {
            let ptr = NativePtr.OffsetMut(buffer.Pointer, (isize) idx);
            * ptr = 97u8;
            idx += 1usize;
        }
        var slice = new ChicStr {
            ptr = NativePtr.AsConstPtr(buffer.Pointer), len = total
        }
        ;
        var str = StringRuntime.chic_rt_string_new();
        if (StringRuntime.chic_rt_string_push_slice (& str, slice) != 0)
        {
            StringRuntime.chic_rt_string_drop(& str);
            NativeAlloc.Free(buffer);
            return false;
        }
        if (StringRuntime.chic_rt_string_get_len (& str) != total)
        {
            StringRuntime.chic_rt_string_drop(& str);
            NativeAlloc.Free(buffer);
            return false;
        }
        if (StringRuntime.chic_rt_string_truncate (& str, inlineCap / 2usize) != 0)
        {
            StringRuntime.chic_rt_string_drop(& str);
            NativeAlloc.Free(buffer);
            return false;
        }
        if (StringRuntime.chic_rt_string_get_len (& str) != inlineCap / 2usize)
        {
            StringRuntime.chic_rt_string_drop(& str);
            NativeAlloc.Free(buffer);
            return false;
        }
        var clone = StringRuntime.chic_rt_string_new();
        if (StringRuntime.chic_rt_string_clone (& clone, & str) != 0)
        {
            StringRuntime.chic_rt_string_drop(& clone);
            StringRuntime.chic_rt_string_drop(& str);
            NativeAlloc.Free(buffer);
            return false;
        }
        if (StringRuntime.chic_rt_string_get_len (& clone) != inlineCap / 2usize)
        {
            StringRuntime.chic_rt_string_drop(& clone);
            StringRuntime.chic_rt_string_drop(& str);
            NativeAlloc.Free(buffer);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& clone);
        StringRuntime.chic_rt_string_drop(& str);
        NativeAlloc.Free(buffer);
        return true;
    }
}
testcase Given_string_setters_and_failure_paths_When_executed_Then_string_setters_and_failure_paths()
{
    unsafe {
        NativeAlloc.TestFailAllocAfter(0);
        var failed = StringRuntime.chic_rt_string_with_capacity(64usize);
        if (StringRuntime.chic_rt_string_get_cap (& failed) != 0usize)
        {
            NativeAlloc.TestReset();
            return false;
        }
        NativeAlloc.TestReset();
        StringRuntime.chic_rt_string_drop((* mut ChicString) NativePtr.NullMut());
        var str = StringRuntime.chic_rt_string_new();
        let inlinePtr = StringRuntime.chic_rt_string_inline_ptr(& str);
        StringRuntime.chic_rt_string_set_ptr(& str, inlinePtr);
        StringRuntime.chic_rt_string_set_len(& str, 0usize);
        StringRuntime.chic_rt_string_set_cap(& str, StringRuntime.chic_rt_string_inline_capacity());
        var badSlice = new ChicStr {
            ptr = NativePtr.NullConst(), len = 4usize
        }
        ;
        let badPush = StringRuntime.chic_rt_string_push_slice(& str, badSlice);
        if (badPush != (int) StringError.InvalidPointer)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        var emptySlice = new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        ;
        let emptyStatus = StringRuntime.chic_rt_string_append_slice(& str, emptySlice, 0, 0);
        if (emptyStatus != (int) StringError.Success)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        let invalidChar = StringRuntime.chic_rt_string_append_char(& str, 0x110000u, 0, 0, emptySlice);
        if (invalidChar != (int) StringError.Utf8)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        let invalidClone = StringRuntime.chic_rt_string_clone_slice((* mut ChicString) NativePtr.NullMut(), emptySlice);
        if (invalidClone != (int) StringError.InvalidPointer)
        {
            StringRuntime.chic_rt_string_drop(& str);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& str);
        return true;
    }
}
testcase Given_string_error_message_capacity_overflow_When_requested_Then_length_matches()
{
    unsafe {
        let msg = StringRuntime.chic_rt_string_error_message((int) StringError.CapacityOverflow);
        if (msg.len != 17usize)
        {
            if (msg.len >0usize)
            {
                NativeAlloc.Free(new ValueMutPtr {
                    Pointer = NativePtr.AsMutPtr(msg.ptr), Size = msg.len, Alignment = 1usize
                }
                );
            }
            return false;
        }
        if (msg.len >0usize)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = NativePtr.AsMutPtr(msg.ptr), Size = msg.len, Alignment = 1usize
            }
            );
        }
        return true;
    }
}
testcase Given_string_error_message_allocation_failed_When_requested_Then_length_matches()
{
    unsafe {
        let msg = StringRuntime.chic_rt_string_error_message((int) StringError.AllocationFailed);
        if (msg.len != 17usize)
        {
            if (msg.len >0usize)
            {
                NativeAlloc.Free(new ValueMutPtr {
                    Pointer = NativePtr.AsMutPtr(msg.ptr), Size = msg.len, Alignment = 1usize
                }
                );
            }
            return false;
        }
        if (msg.len >0usize)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = NativePtr.AsMutPtr(msg.ptr), Size = msg.len, Alignment = 1usize
            }
            );
        }
        return true;
    }
}
testcase Given_string_error_message_out_of_bounds_When_requested_Then_length_matches()
{
    unsafe {
        let msg = StringRuntime.chic_rt_string_error_message((int) StringError.OutOfBounds);
        if (msg.len != 13usize)
        {
            if (msg.len >0usize)
            {
                NativeAlloc.Free(new ValueMutPtr {
                    Pointer = NativePtr.AsMutPtr(msg.ptr), Size = msg.len, Alignment = 1usize
                }
                );
            }
            return false;
        }
        if (msg.len >0usize)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = NativePtr.AsMutPtr(msg.ptr), Size = msg.len, Alignment = 1usize
            }
            );
        }
        return true;
    }
}
testcase Given_string_push_slice_allocation_failure_When_alloc_fails_Then_returns_allocation_failed()
{
    unsafe {
        let inlineCap = StringRuntime.chic_rt_string_inline_capacity();
        var buffer = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = inlineCap + 4usize, Alignment = 1usize
        }
        ;
        let alloc = NativeAlloc.AllocZeroed(buffer.Size, buffer.Alignment, out buffer);
        NativeAlloc.TestFailAllocAfter(0);
        var str = StringRuntime.chic_rt_string_new();
        let slice = new ChicStr {
            ptr = NativePtr.AsConstPtr(buffer.Pointer), len = buffer.Size
        }
        ;
        let status = StringRuntime.chic_rt_string_push_slice(& str, slice);
        NativeAlloc.TestReset();
        if (alloc != NativeAllocationError.Success)
        {
            StringRuntime.chic_rt_string_drop(& str);
            NativeAlloc.Free(buffer);
            return false;
        }
        if (status != (int) StringError.AllocationFailed)
        {
            StringRuntime.chic_rt_string_drop(& str);
            NativeAlloc.Free(buffer);
            return false;
        }
        StringRuntime.chic_rt_string_drop(& str);
        NativeAlloc.Free(buffer);
        return true;
    }
}
testcase Given_string_internal_helpers_coverage_When_executed_Then_string_internal_helpers_coverage()
{
    unsafe {
        StringRuntime.TestCoverageHelpers();
        return true;
    }
}
testcase Given_string_format_precision_and_padding_When_executed_Then_string_format_precision_and_padding()
{
    unsafe {
        var str = StringRuntime.chic_rt_string_new();
        var ok = true;
        var fmtHex = new StringInlineBytes32 {
            b00 = 88, b01 = 52, b02 = 0,
        }
        ;
        let hexSlice = new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtHex.b00), len = 2usize
        }
        ;
        let hexStatus = StringRuntime.chic_rt_string_append_unsigned(& str, 0xABu64, 0u64, 32u32, 6, 1, hexSlice);
        ok = ok && hexStatus == 0;
        var fmtHexLower = new StringInlineBytes32 {
            b00 = 120, b01 = 50, b02 = 0,
        }
        ;
        let hexLower = new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtHexLower.b00), len = 2usize
        }
        ;
        let signedStatus = StringRuntime.chic_rt_string_append_signed(& str, - 255, - 1, 64u32, - 6, 1, hexLower);
        ok = ok && signedStatus == 0;
        var fmtExp = new StringInlineBytes32 {
            b00 = 69, b01 = 51, b02 = 0,
        }
        ;
        let expSlice = new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtExp.b00), len = 2usize
        }
        ;
        let expStatus = StringRuntime.chic_rt_string_append_f64(& str, 1234.5, 0, 0, expSlice);
        ok = ok && expStatus == 0;
        var fmtGen = new StringInlineBytes32 {
            b00 = 71, b01 = 52, b02 = 0,
        }
        ;
        let genSlice = new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtGen.b00), len = 2usize
        }
        ;
        let genStatus = StringRuntime.chic_rt_string_append_f64(& str, 0.000123, 0, 0, genSlice);
        ok = ok && genStatus == 0;
        var fmtFix = new StringInlineBytes32 {
            b00 = 102, b01 = 50, b02 = 0,
        }
        ;
        let fixSlice = new ChicStr {
            ptr = NativePtr.AsConstPtr(& fmtFix.b00), len = 2usize
        }
        ;
        let fixStatus = StringRuntime.chic_rt_string_append_f16(& str, 123u16, 0, 0, fixSlice);
        ok = ok && fixStatus == 0;
        let outSlice = StringRuntime.chic_rt_string_as_slice(& str);
        ok = ok && outSlice.len >0usize;
        StringRuntime.chic_rt_string_drop(& str);
        return ok;
    }
}
