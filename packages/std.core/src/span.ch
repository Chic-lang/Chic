namespace Std.Span;
import Std.Runtime.Collections;
import Std.Memory;
import Std.Range;
import Std.Core;
import Std.Numeric;
import Std.Core.Testing;
public enum SpanError
{
    Success = 0, NullPointer = 1, OutOfBounds = 2, InvalidStride = 3,
}
internal static class SpanIntrinsics
{
    @extern("C") public static extern SpanPtr chic_rt_span_from_raw_mut(ref ValueMutPtr data, usize length);
    @extern("C") public static extern ReadOnlySpanPtr chic_rt_span_from_raw_const(ref ValueConstPtr data, usize length);
    @extern("C") public static extern int chic_rt_span_slice_mut(ref SpanPtr source, usize start, usize length, out SpanPtr destination);
    @extern("C") public static extern int chic_rt_span_slice_readonly(ref ReadOnlySpanPtr source, usize start, usize length,
    out ReadOnlySpanPtr destination);
    @extern("C") public static extern ReadOnlySpanPtr chic_rt_span_to_readonly(ref SpanPtr source);
    @extern("C") public static extern int chic_rt_span_copy_to(ref ReadOnlySpanPtr source, ref SpanPtr destination);
    @extern("C") public static extern * mut @expose_address byte chic_rt_span_ptr_at_mut(ref SpanPtr source, usize index);
    @extern("C") public static extern * const @readonly @expose_address byte chic_rt_span_ptr_at_readonly(ref ReadOnlySpanPtr source,
    usize index);
    @extern("C") public static extern StrPtr chic_rt_string_as_slice(* const @readonly string value);
    @extern("C") public static extern string chic_rt_string_from_slice(StrPtr slice);
    @extern("C") public static extern CharSpanPtr chic_rt_string_as_chars(* const @readonly string value);
    @extern("C") public static extern CharSpanPtr chic_rt_str_as_chars(StrPtr slice);
}
internal static class SpanGuards
{
    public static void AssertStatus(int status) {
        if (status == (int) SpanError.Success)
        {
            return;
        }
        if (status == (int) SpanError.OutOfBounds)
        {
            throw new Std.IndexOutOfRangeException("span operation exceeded bounds");
        }
        if (status == (int) SpanError.NullPointer)
        {
            throw new Std.ArgumentNullException("span referenced a null pointer");
        }
        if (status == (int) SpanError.InvalidStride)
        {
            throw new Std.ArgumentException("span stride or length was invalid");
        }
        throw new Std.InvalidOperationException("span operation failed");
    }
    public static RangeBounds ResolveRangeExclusive(Range range, usize length) {
        let bounds = RangeMath.Resolve(range, length);
        return bounds;
    }
    public static RangeBounds ResolveRangeInclusive(RangeInclusive range, usize length) {
        let bounds = RangeMath.ResolveInclusive(range, length);
        return bounds;
    }
    public static RangeBounds ResolveRangeFrom(RangeFrom range, usize length) {
        let bounds = RangeMath.ResolveFrom(range, length);
        return bounds;
    }
    public static RangeBounds ResolveRangeTo(RangeTo range, usize length) {
        let bounds = RangeMath.ResolveTo(range, length);
        return bounds;
    }
    public static RangeBounds ResolveFull(usize length) {
        return RangeMath.ResolveFull(length);
    }
}
internal static class SpanHelpers
{
    public static SpanPtr SliceMut(ref SpanPtr source, usize start, usize length) {
        var destination = new SpanPtr();
        let status = SpanIntrinsics.chic_rt_span_slice_mut(ref source, start, length, out destination);
        SpanGuards.AssertStatus(status);
        return destination;
    }
    public static ReadOnlySpanPtr SliceReadonly(ref ReadOnlySpanPtr source, usize start, usize length) {
        var destination = new ReadOnlySpanPtr();
        let status = SpanIntrinsics.chic_rt_span_slice_readonly(ref source, start, length, out destination);
        SpanGuards.AssertStatus(status);
        return destination;
    }
}
public struct Span <T >
{
    internal SpanPtr Raw;
    private static void AssertHandle(ValueMutPtr handle) {
        let expectedSize = __sizeof <T >();
        let expectedAlignment = __alignof <T >();
        if (expectedSize != 0 && (handle.Size != expectedSize || handle.Alignment != expectedAlignment))
        {
            throw new Std.InvalidOperationException("Span requires a ValueMutPtr sized/aligned for the span element type");
        }
        return;
    }
    private static void AssertRaw(SpanPtr raw) {
        AssertHandle(raw.Data);
        let expectedSize = __sizeof <T >();
        let expectedAlignment = __alignof <T >();
        if (expectedSize != 0 && (raw.ElementSize != expectedSize || raw.ElementAlignment != expectedAlignment))
        {
            throw new Std.InvalidOperationException("Span requires a raw span sized/aligned for the span element type");
        }
        return;
    }
    private static SpanPtr CreateRaw(ValueMutPtr handle, usize length) {
        AssertHandle(handle);
        var temp = handle;
        let raw = SpanIntrinsics.chic_rt_span_from_raw_mut(ref temp, length);
        AssertRaw(raw);
        return raw;
    }
    public init(SpanPtr raw) {
        AssertRaw(raw);
        Raw = raw;
    }
    public usize Length => Raw.Length;
    public bool IsEmpty => Raw.Length == 0;
    public static Span <T >Empty {
        get {
            let handle = ValuePointer.NullMut(__sizeof <T >(), __alignof <T >());
            return FromValuePointer(handle, 0);
        }
    }
    public static Span <T >FromValuePointer(ValueMutPtr handle, usize length) {
        return new Span <T >(CreateRaw(handle, length));
    }
    public static Span <T >FromVec(ref VecPtr vec) {
        unsafe {
            let handle = ValuePointer.CreateMut(PointerIntrinsics.AsByteMut(vec.Pointer), vec.ElementSize, vec.ElementAlignment);
            return FromValuePointer(handle, vec.Length);
        }
    }
    public static Span <T >FromArray(ref ArrayPtr array) {
        unsafe {
            let handle = ValuePointer.CreateMut(PointerIntrinsics.AsByteMut(array.Pointer), array.ElementSize, array.ElementAlignment);
            return FromValuePointer(handle, array.Length);
        }
    }
    public static Span <T >StackAlloc(usize length) {
        let elementSize = __sizeof <T >();
        let elementAlignment = __alignof <T >();
        if (elementSize == 0)
        {
            let handle = ValuePointer.NullMut(elementSize, elementAlignment);
            return FromValuePointer(handle, length);
        }
        let totalSize = elementSize * length;
        var raw = ValuePointer.NullMut(elementSize, elementAlignment);
        let status = Std.Memory.Memory.Alloc(totalSize, elementAlignment, out raw);
        if ((int) status != (int) Std.Memory.AllocationError.Success)
        {
            throw new Std.InvalidOperationException("allocation failed while constructing span");
        }
        unsafe {
            let handle = ValuePointer.CreateMut(PointerIntrinsics.AsByteMut(raw.Pointer), elementSize, elementAlignment);
            return FromValuePointer(handle, length);
        }
    }
    public static Span <T >StackAlloc(ReadOnlySpan <T >source) {
        var scratch = Span <T >.StackAlloc(source.Length);
        scratch.CopyFrom(source);
        return scratch;
    }
    public Span <T >Slice(usize start, usize length) {
        var raw = Raw;
        var sliced = SpanHelpers.SliceMut(ref raw, start, length);
        return new Span <T >(sliced);
    }
    public Span <T >Slice(usize start) {
        return Slice(start, Raw.Length - start);
    }
    public Span <T >Slice(Range range) {
        let bounds = SpanGuards.ResolveRangeExclusive(range, Raw.Length);
        return Slice(bounds.Start, bounds.Length);
    }
    public Span <T >Slice(RangeInclusive range) {
        let bounds = SpanGuards.ResolveRangeInclusive(range, Raw.Length);
        return Slice(bounds.Start, bounds.Length);
    }
    public Span <T >Slice(RangeFrom range) {
        let bounds = SpanGuards.ResolveRangeFrom(range, Raw.Length);
        return Slice(bounds.Start, bounds.Length);
    }
    public Span <T >Slice(RangeTo range) {
        let bounds = SpanGuards.ResolveRangeTo(range, Raw.Length);
        return Slice(bounds.Start, bounds.Length);
    }
    public Span <T >Slice(RangeFull _) {
        return Slice(0, Raw.Length);
    }
    public ReadOnlySpan <T >AsReadOnly() {
        var raw = Raw;
        var ro = SpanIntrinsics.chic_rt_span_to_readonly(ref raw);
        return new ReadOnlySpan <T >(ro);
    }
    public void CopyTo(Span <T >destination) {
        AsReadOnly().CopyTo(destination);
        return;
    }
    public void CopyFrom(ReadOnlySpan <T >source) {
        let elementSize = Raw.Data.Size;
        if (elementSize == 0)
        {
            return;
        }
        let count = source.Length <Raw.Length ?source.Length : Raw.Length;
        if (count == 0)
        {
            return;
        }
        let byteLen = count * elementSize;
        Std.Memory.GlobalAllocator.Copy(Raw.Data, source.Raw.Data, byteLen);
        return;
    }
}
public struct ReadOnlySpan <T >
{
    internal ReadOnlySpanPtr Raw;
    private static void AssertHandle(ValueConstPtr handle) {
        let expectedSize = __sizeof <T >();
        let expectedAlignment = __alignof <T >();
        if (expectedSize != 0 && (handle.Size != expectedSize || handle.Alignment != expectedAlignment))
        {
            throw new Std.InvalidOperationException("ReadOnlySpan requires a ValueConstPtr sized/aligned for the span element type");
        }
        return;
    }
    private static void AssertRaw(ReadOnlySpanPtr raw) {
        AssertHandle(raw.Data);
        let expectedSize = __sizeof <T >();
        let expectedAlignment = __alignof <T >();
        if (expectedSize != 0 && (raw.ElementSize != expectedSize || raw.ElementAlignment != expectedAlignment))
        {
            throw new Std.InvalidOperationException("ReadOnlySpan requires a raw span sized/aligned for the span element type");
        }
        return;
    }
    private static ReadOnlySpanPtr CreateRaw(ValueConstPtr handle, usize length) {
        AssertHandle(handle);
        var temp = handle;
        let raw = SpanIntrinsics.chic_rt_span_from_raw_const(ref temp, length);
        AssertRaw(raw);
        return raw;
    }
    public init(ReadOnlySpanPtr raw) {
        AssertRaw(raw);
        Raw = raw;
    }
    public usize Length => Raw.Length;
    public bool IsEmpty => Raw.Length == 0;
    public static ReadOnlySpan <T >Empty {
        get {
            let handle = ValuePointer.NullConst(__sizeof <T >(), __alignof <T >());
            return FromValuePointer(handle, 0);
        }
    }
    public static ReadOnlySpan <T >FromValuePointer(ValueConstPtr handle, usize length) {
        return new ReadOnlySpan <T >(CreateRaw(handle, length));
    }
    public static ReadOnlySpan <T >FromVec(in VecPtr vec) {
        unsafe {
            let handle = ValuePointer.CreateConst(PointerIntrinsics.AsByteConstFromMut(vec.Pointer), vec.ElementSize, vec.ElementAlignment);
            return FromValuePointer(handle, vec.Length);
        }
    }
    public static ReadOnlySpan <T >FromArray(in ArrayPtr array) {
        unsafe {
            let handle = ValuePointer.CreateConst(PointerIntrinsics.AsByteConstFromMut(array.Pointer), array.ElementSize,
            array.ElementAlignment);
            return FromValuePointer(handle, array.Length);
        }
    }
    public ReadOnlySpan <T >Slice(usize start, usize length) {
        var raw = Raw;
        var sliced = SpanHelpers.SliceReadonly(ref raw, start, length);
        return new ReadOnlySpan <T >(sliced);
    }
    public ReadOnlySpan <T >Slice(usize start) {
        return Slice(start, Raw.Length - start);
    }
    public ReadOnlySpan <T >Slice(Range range) {
        let bounds = SpanGuards.ResolveRangeExclusive(range, Raw.Length);
        return Slice(bounds.Start, bounds.Length);
    }
    public ReadOnlySpan <T >Slice(RangeInclusive range) {
        let bounds = SpanGuards.ResolveRangeInclusive(range, Raw.Length);
        return Slice(bounds.Start, bounds.Length);
    }
    public ReadOnlySpan <T >Slice(RangeFrom range) {
        let bounds = SpanGuards.ResolveRangeFrom(range, Raw.Length);
        return Slice(bounds.Start, bounds.Length);
    }
    public ReadOnlySpan <T >Slice(RangeTo range) {
        let bounds = SpanGuards.ResolveRangeTo(range, Raw.Length);
        return Slice(bounds.Start, bounds.Length);
    }
    public ReadOnlySpan <T >Slice(RangeFull _) {
        return Slice(0, Raw.Length);
    }
    public void CopyTo(Span <T >destination) {
        let elementSize = Raw.Data.Size;
        if (elementSize == 0)
        {
            return;
        }
        let count = destination.Length <Raw.Length ?destination.Length : Raw.Length;
        if (count == 0)
        {
            return;
        }
        let byteLen = count * elementSize;
        Std.Memory.GlobalAllocator.Copy(destination.Raw.Data, Raw.Data, byteLen);
        return;
    }
}
public static class ReadOnlySpan
{
    @extern("C") private static extern CharSpanPtr chic_rt_string_as_chars(* const @readonly string value);
    @extern("C") private static extern CharSpanPtr chic_rt_str_as_chars(StrPtr slice);
    public static ReadOnlySpan <byte >FromString(string value) {
        let slice = SpanIntrinsics.chic_rt_string_as_slice(& value);
        let handle = ValuePointer.CreateConst(PointerIntrinsics.AsByteConst(slice.Pointer), 1, 1);
        return ReadOnlySpan <byte >.FromValuePointer(handle, slice.Length);
    }
    public static ReadOnlySpan <char >FromStringChars(string value) {
        let slice = chic_rt_string_as_chars(& value);
        let elementSize = __sizeof <char >();
        let elementAlignment = __alignof <char >();
        let handle = ValuePointer.CreateConst(PointerIntrinsics.AsByteConst(slice.Pointer), elementSize, elementAlignment);
        return ReadOnlySpan <char >.FromValuePointer(handle, slice.Length);
    }
    public static ReadOnlySpan <char >FromStr(StrPtr slice) {
        let chars = chic_rt_str_as_chars(slice);
        let elementSize = __sizeof <char >();
        let elementAlignment = __alignof <char >();
        let handle = ValuePointer.CreateConst(PointerIntrinsics.AsByteConst(chars.Pointer), elementSize, elementAlignment);
        return ReadOnlySpan <char >.FromValuePointer(handle, chars.Length);
    }
}

static class SpanArrayTestHelpers
{
    public static ArrayPtr CreateArray(usize length, out ValueMutPtr handle) {
        let status = Std.Memory.Memory.Alloc(__sizeof<int>() * length, __alignof<int>(), out handle);
        Assert.That(status == Std.Memory.AllocationError.Success).IsTrue();
        return new ArrayPtr {
            Pointer = handle.Pointer,
            Length = length,
            Capacity = length,
            ElementSize = __sizeof<int>(),
            ElementAlignment = __alignof<int>(),
            DropCallback = 0isize,
        };
    }
    public static void Free(ValueMutPtr handle) {
        Std.Memory.Memory.Free(handle);
    }
}

testcase Given_span_empty_length_zero_When_executed_Then_span_empty_length_zero()
{
    let span = Span<byte>.Empty;
    Assert.That(span.Length == 0usize).IsTrue();
}

testcase Given_span_empty_is_empty_true_When_executed_Then_span_empty_is_empty_true()
{
    let span = Span<byte>.Empty;
    Assert.That(span.IsEmpty).IsTrue();
}

testcase Given_span_stackalloc_length_When_executed_Then_span_stackalloc_length()
{
    var span = Span<byte>.StackAlloc(4usize);
    Assert.That(span.Length == 4usize).IsTrue();
}

testcase Given_span_slice_length_When_executed_Then_span_slice_length()
{
    var span = Span<byte>.StackAlloc(4usize);
    let slice = span.Slice(1usize, 2usize);
    Assert.That(slice.Length == 2usize).IsTrue();
}

testcase Given_span_slice_range_exclusive_length_When_executed_Then_span_slice_range_exclusive_length()
{
    var span = Span<byte>.StackAlloc(5usize);
    var range = new Range();
    range.Start = Index.FromStart(1usize);
    range.End = Index.FromEnd(1usize);
    let slice = span.Slice(range);
    Assert.That(slice.Length == 3usize).IsTrue();
}

testcase Given_span_slice_inclusive_range_length_When_executed_Then_span_slice_inclusive_range_length()
{
    var span = Span<byte>.StackAlloc(4usize);
    var range = new RangeInclusive();
    range.Start = Index.FromStart(1usize);
    range.End = Index.FromStart(2usize);
    let slice = span.Slice(range);
    Assert.That(slice.Length == 2usize).IsTrue();
}

testcase Given_span_slice_range_out_of_bounds_throws_When_executed_Then_span_slice_range_out_of_bounds_throws()
{
    var span = Span<byte>.StackAlloc(2usize);
    var threw = false;
    try {
        var range = new Range();
        range.Start = Index.FromStart(0usize);
        range.End = Index.FromStart(3usize);
        let _ = span.Slice(range);
    }
    catch(ArgumentOutOfRangeException) {
        threw = true;
    }
    Assert.That(threw).IsTrue();
}

testcase Given_span_slice_index_out_of_bounds_throws_When_executed_Then_span_slice_index_out_of_bounds_throws()
{
    var span = Span<byte>.StackAlloc(2usize);
    var threw = false;
    try {
        let _ = span.Slice(1usize, 4usize);
    }
    catch(IndexOutOfRangeException) {
        threw = true;
    }
    Assert.That(threw).IsTrue();
}

testcase Given_readonly_span_from_string_bytes_length_When_executed_Then_readonly_span_from_string_bytes_length()
{
    let text = "hello";
    let bytes = ReadOnlySpan.FromString(text);
    Assert.That(bytes.Length == 5usize).IsTrue();
}

testcase Given_string_from_slice_roundtrip_bytes_length_When_executed_Then_string_from_slice_roundtrip_bytes_length()
{
    let slice = StrPtr.FromStr("hello");
    let text = SpanIntrinsics.chic_rt_string_from_slice(slice);
    let textSlice = SpanIntrinsics.chic_rt_string_as_slice(& text);
    Assert.That(textSlice.Length == 5usize).IsTrue();
}

testcase Given_readonly_span_from_string_chars_length_When_executed_Then_readonly_span_from_string_chars_length()
{
    let text = "hello";
    let chars = ReadOnlySpan.FromStringChars(text);
    Assert.That(chars.Length == 5usize).IsTrue();
}

testcase Given_readonly_span_copy_to_span_length_matches_When_executed_Then_readonly_span_copy_to_span_length_matches()
{
    let text = "abc";
    let bytes = ReadOnlySpan.FromString(text);
    var dest = Span<byte>.StackAlloc(bytes.Length);
    bytes.CopyTo(dest);
    Assert.That(dest.Length == bytes.Length).IsTrue();
}

testcase Given_span_stackalloc_from_readonly_span_length_When_executed_Then_span_stackalloc_from_readonly_span_length()
{
    let bytes = ReadOnlySpan.FromString("data");
    let copy = Span<byte>.StackAlloc(bytes);
    Assert.That(copy.Length == bytes.Length).IsTrue();
}

testcase Given_readonly_span_slice_from_length_When_executed_Then_readonly_span_slice_from_length()
{
    let bytes = ReadOnlySpan.FromString("hello");
    var from = new RangeFrom();
    from.Start = Index.FromStart(2usize);
    let slice_from = bytes.Slice(from);
    Assert.That(slice_from.Length == 3usize).IsTrue();
}

testcase Given_readonly_span_slice_to_length_When_executed_Then_readonly_span_slice_to_length()
{
    let bytes = ReadOnlySpan.FromString("hello");
    var to = new RangeTo();
    to.End = Index.FromStart(3usize);
    let slice_to = bytes.Slice(to);
    Assert.That(slice_to.Length == 3usize).IsTrue();
}

testcase Given_readonly_span_slice_full_length_When_executed_Then_readonly_span_slice_full_length()
{
    let bytes = ReadOnlySpan.FromString("hello");
    let full = new RangeFull();
    let slice_full = bytes.Slice(full);
    Assert.That(slice_full.Length == bytes.Length).IsTrue();
}

testcase Given_span_from_value_pointer_invalid_layout_throws_When_executed_Then_span_from_value_pointer_invalid_layout_throws()
{
    let handle = ValuePointer.NullMut(1usize, 1usize);
    var threw = false;
    try {
        let _ = Span<int>.FromValuePointer(handle, 1usize);
    }
    catch(InvalidOperationException) {
        threw = true;
    }
    Assert.That(threw).IsTrue();
}

testcase Given_span_copy_from_sets_middle_value_When_executed_Then_span_copy_from_sets_middle_value()
{
    var left = Span<int>.StackAlloc(3usize);
    left[0usize] = 2;
    left[1usize] = 4;
    left[2usize] = 6;
    var right = Span<int>.StackAlloc(3usize);
    right[0usize] = 9;
    right[1usize] = 9;
    right[2usize] = 9;
    right.CopyFrom(left.AsReadOnly());
    Assert.That(right[1usize] == 4).IsTrue();
}

testcase Given_span_copy_from_readonly_sets_end_value_When_executed_Then_span_copy_from_readonly_sets_end_value()
{
    var left = Span<int>.StackAlloc(3usize);
    left[0usize] = 2;
    left[1usize] = 4;
    left[2usize] = 6;
    var right = Span<int>.StackAlloc(3usize);
    right[0usize] = 9;
    right[1usize] = 9;
    right[2usize] = 9;
    right.CopyFrom(left.AsReadOnly());
    let ro = right.AsReadOnly();
    var copy = Span<int>.StackAlloc(ro.Length);
    copy.CopyFrom(ro);
    Assert.That(copy[2usize] == 6).IsTrue();
}

testcase Given_readonly_span_from_strptr_length_When_executed_Then_readonly_span_from_strptr_length()
{
    let slice = StrPtr.FromStr("span");
    let chars = ReadOnlySpan.FromStr(slice);
    Assert.That(chars.Length == 4usize).IsTrue();
}

testcase Given_span_from_array_alloc_status_success_When_executed_Then_span_from_array_alloc_status_success()
{
    var handle = ValuePointer.NullMut(__sizeof<int>(), __alignof<int>());
    let status = Std.Memory.Memory.Alloc(__sizeof<int>() * 2usize, __alignof<int>(), out handle);
    Assert.That(status == Std.Memory.AllocationError.Success).IsTrue();
    Std.Memory.Memory.Free(handle);
}

testcase Given_span_from_array_length_When_executed_Then_span_from_array_length()
{
    var handle = ValuePointer.NullMut(__sizeof<int>(), __alignof<int>());
    var array = SpanArrayTestHelpers.CreateArray(2usize, out handle);
    let span = Span<int>.FromArray(ref array);
    Assert.That(span.Length == 2usize).IsTrue();
    SpanArrayTestHelpers.Free(handle);
}

testcase Given_readonly_span_from_array_length_When_executed_Then_readonly_span_from_array_length()
{
    var handle = ValuePointer.NullMut(__sizeof<int>(), __alignof<int>());
    var array = SpanArrayTestHelpers.CreateArray(2usize, out handle);
    let readonlySpan = ReadOnlySpan<int>.FromArray(in array);
    Assert.That(readonlySpan.Length == 2usize).IsTrue();
    SpanArrayTestHelpers.Free(handle);
}
