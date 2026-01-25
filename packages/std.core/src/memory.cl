namespace Std.Memory;
import Std.Numeric;
import Std.Runtime;
import Std.Runtime.Collections;
import Std.Span;
import Std.Core;
import Std.Core.Testing;
public enum AllocationError
{
    Success = 0, InvalidLayout = 1, AllocationFailed = 2,
}
public static class Intrinsics
{
    public static T ZeroValue <T >() {
        ZeroInit <T >(out var value);
        return value;
    }
    public static void ZeroInit <T >(out T target) {
        target = ZeroValue <T >();
    }
    public unsafe static void ZeroInitRaw(* mut @expose_address byte pointer, usize length) {
        unsafe {
            if (Std.Numeric.Pointer.IsNull (pointer) || length == 0)
            {
                return;
            }
        }
        let handle = Std.Runtime.Collections.ValuePointer.CreateMut(Std.Numeric.PointerIntrinsics.AsByteMut(pointer), length,
        1);
        GlobalAllocator.Set(handle, 0, length);
    }
}
/// <summary>
/// Thin wrapper over the host allocator used by the runtime. These helpers intentionally
/// mirror the `chic_rt_*` exports so higher-level Chic code can remain allocator
/// agnostic while we phase out the Rust shims.
/// </summary>
public static class GlobalAllocator
{
    @extern("C") private static extern ValueMutPtr chic_rt_alloc(usize size, usize align);
    @extern("C") private static extern ValueMutPtr chic_rt_alloc_zeroed(usize size, usize align);
    @extern("C") private static extern ValueMutPtr chic_rt_realloc(ValueMutPtr ptr, usize oldSize, usize newSize, usize align);
    @extern("C") private static extern void chic_rt_free(ValueMutPtr ptr);
    @extern("C") private static extern void chic_rt_memcpy(ValueMutPtr dest, ValueConstPtr src, usize len);
    @extern("C") private static extern void chic_rt_memmove(ValueMutPtr dest, ValueMutPtr src, usize len);
    @extern("C") private static extern void chic_rt_memset(ValueMutPtr dest, byte value, usize len);
    public static AllocationError Alloc(usize size, usize align, out ValueMutPtr result) {
        let handle = chic_rt_alloc(size, align);
        result = handle;
        var isNull = false;
        unsafe {
            isNull = Std.Numeric.Pointer.IsNull(handle.Pointer);
        }
        return isNull ?AllocationError.AllocationFailed : AllocationError.Success;
    }
    public static AllocationError AllocZeroed(usize size, usize align, out ValueMutPtr result) {
        let handle = chic_rt_alloc_zeroed(size, align);
        result = handle;
        var isNull = false;
        unsafe {
            isNull = Std.Numeric.Pointer.IsNull(handle.Pointer);
        }
        return isNull ?AllocationError.AllocationFailed : AllocationError.Success;
    }
    public static AllocationError Realloc(ValueMutPtr pointer, usize oldSize, usize newSize, usize align, out ValueMutPtr result) {
        let updated = chic_rt_realloc(pointer, oldSize, newSize, align);
        result = updated;
        var isNull = false;
        unsafe {
            isNull = Std.Numeric.Pointer.IsNull(updated.Pointer);
        }
        return isNull ?AllocationError.AllocationFailed : AllocationError.Success;
    }
    public static void Free(ValueMutPtr pointer) {
        chic_rt_free(pointer);
    }
    public static void Copy(ValueMutPtr destination, ValueConstPtr source, usize length) {
        chic_rt_memcpy(destination, source, length);
    }
    public static void Move(ValueMutPtr destination, ValueMutPtr source, usize length) {
        chic_rt_memmove(destination, source, length);
    }
    public static void Set(ValueMutPtr destination, byte value, usize length) {
        chic_rt_memset(destination, value, length);
    }
    public static ValueMutPtr Offset(ValueMutPtr pointer, isize bytes) {
        var updated = pointer;
        unsafe {
            if (pointer.Pointer != null && bytes != 0)
            {
                let baseAddr = (isize) pointer.Pointer;
                updated.Pointer = (* mut @expose_address byte)(baseAddr + bytes);
            }
        }
        return updated;
    }
    public static void InitializeDefault <T >(out T target) {
        Intrinsics.ZeroInit(out target);
    }
}
public static class Memory
{
    private static ValueMutPtr NullMut() {
        return Std.Runtime.Collections.ValuePointer.NullMut(0usize, 0usize);
    }
    public static AllocationError Alloc(usize size, usize align, out ValueMutPtr result) {
        var temp = NullMut();
        let status = GlobalAllocator.Alloc(size, align, out temp);
        result = temp;
        return status;
    }
    public static AllocationError AllocZeroed(usize size, usize align, out ValueMutPtr result) {
        var temp = NullMut();
        let status = GlobalAllocator.AllocZeroed(size, align, out temp);
        result = temp;
        return status;
    }
    public static AllocationError Realloc(ValueMutPtr pointer, usize oldSize, usize newSize, usize align, out ValueMutPtr result) {
        var temp = NullMut();
        let status = GlobalAllocator.Realloc(pointer, oldSize, newSize, align, out temp);
        result = temp;
        return status;
    }
    public static void Free(ValueMutPtr pointer) {
        GlobalAllocator.Free(pointer);
    }
    public static void Copy(ValueMutPtr destination, ValueConstPtr source, usize length) {
        GlobalAllocator.Copy(destination, source, length);
    }
    public static void Move(ValueMutPtr destination, ValueMutPtr source, usize length) {
        GlobalAllocator.Move(destination, source, length);
    }
    public static void Set(ValueMutPtr destination, byte value, usize length) {
        GlobalAllocator.Set(destination, value, length);
    }
    public static ValueMutPtr Offset(ValueMutPtr pointer, isize bytes) {
        return GlobalAllocator.Offset(pointer, bytes);
    }
    public static void InitializeDefault <T >(out T target) {
        Intrinsics.ZeroInit(out target);
    }
}
public static class StackAlloc
{
    public static Std.Span.Span <T >Span <T >(usize length) {
        return Std.Span.Span <T >.StackAlloc(length);
    }
    public static Std.Span.Span <T >Span <T >(Std.Span.ReadOnlySpan <T >source) {
        return Std.Span.Span <T >.StackAlloc(source);
    }
    public static ValueMutPtr FromSpan <T >(Std.Span.Span <T >span) {
        return Std.Runtime.Collections.ValuePointer.CreateMut(Std.Numeric.PointerIntrinsics.AsByteMut(span.Raw.Data.Pointer),
        span.Raw.Data.Size, span.Raw.Data.Alignment);
    }
    public static ValueConstPtr FromSpan <T >(Std.Span.ReadOnlySpan <T >span) {
        unsafe {
            let elementSize = span.Raw.Data.Size;
            let elementAlignment = span.Raw.Data.Alignment;
            return Std.Runtime.Collections.ValuePointer.CreateConst(Std.Numeric.PointerIntrinsics.AsByteConst(span.Raw.Data.Pointer),
            elementSize, elementAlignment);
        }
    }
}
public struct MaybeUninit <T >
{
    private T _value;
    private bool _initialized;
    private static Std.Runtime.TypeMetadataRecord Metadata() {
        let typeId = __type_id_of <T >();
        return TypeMetadata.Resolve(typeId);
    }
    public static MaybeUninit <T >Uninit() {
        var slot = new MaybeUninit <T >();
        slot._initialized = false;
        return slot;
    }
    public static MaybeUninit <T >Init(T value) {
        var slot = new MaybeUninit <T >();
        slot._value = value;
        slot._initialized = true;
        return slot;
    }
    public static MaybeUninit <T >CreateZeroed() {
        var slot = Uninit();
        let metadata = Metadata();
        let handle = slot.AsValueMutPtr();
        Std.Memory.GlobalAllocator.Set(handle, 0, metadata.Size);
        slot.MarkInitialized();
        return slot;
    }
    public bool IsInitialized() {
        return _initialized;
    }
    public void Write(ref this, T value) {
        if (IsInitialized ())
        {
            throw new Std.InvalidOperationException("value already initialised");
        }
        _value = value;
        _initialized = true;
        return;
    }
    public void ForgetInit(ref this) {
        _initialized = false;
    }
    public void MarkInitialized(ref this) {
        _initialized = true;
    }
    public unsafe T AssumeInit(ref this) {
        if (!IsInitialized ())
        {
            throw new Std.InvalidOperationException("value is not initialised");
        }
        var value = _value;
        _initialized = false;
        return value;
    }
    public unsafe T AssumeInitRead() {
        if (!IsInitialized ())
        {
            throw new Std.InvalidOperationException("value is not initialised");
        }
        return _value;
    }
    public ref T AssumeInitRef(ref this) {
        if (!IsInitialized ())
        {
            throw new Std.InvalidOperationException("value is not initialised");
        }
        return ref _value;
    }
    public Std.Runtime.Collections.ValueMutPtr AsValueMutPtr() {
        let size = (usize) __sizeof <T >();
        let align = (usize) __alignof <T >();
        unsafe {
            var * mut @expose_address T valuePtr = & _value;
            let bytes = Std.Numeric.PointerIntrinsics.AsByteMut(valuePtr);
            return Std.Runtime.Collections.ValuePointer.CreateMut(bytes, size, align);
        }
    }
    public Std.Runtime.Collections.ValueConstPtr AsValueConstPtr() {
        let size = (usize) __sizeof <T >();
        let align = (usize) __alignof <T >();
        unsafe {
            var * mut @expose_address T valuePtr = & _value;
            let bytes = Std.Numeric.PointerIntrinsics.AsByteConstFromMut(valuePtr);
            return Std.Runtime.Collections.ValuePointer.CreateConst(bytes, size, align);
        }
    }
    public void dispose(ref this) {
        if (!IsInitialized ())
        {
            return;
        }
        unsafe {
            var dropFn = (isize) __drop_glue_of <T >();
            Std.Runtime.DropRuntime.Invoke(dropFn, AsValueMutPtr());
        }
        _initialized = false;
    }
}
static class MemoryTestHelpers
{
    public static ValueMutPtr Alloc(usize size, usize align) {
        var handle = ValuePointer.NullMut(0usize, 0usize);
        let status = Memory.Alloc(size, align, out handle);
        if (status != AllocationError.Success)
        {
            throw new InvalidOperationException("allocation failed");
        }
        return handle;
    }
    public static ValueMutPtr AllocZeroed(usize size, usize align) {
        var handle = ValuePointer.NullMut(0usize, 0usize);
        let status = Memory.AllocZeroed(size, align, out handle);
        if (status != AllocationError.Success)
        {
            throw new InvalidOperationException("allocation failed");
        }
        return handle;
    }
    public static Span <byte >SpanFrom(ValueMutPtr handle, usize length) {
        let byteHandle = ValuePointer.CreateMut(handle.Pointer, 1usize, 1usize);
        return Span <byte >.FromValuePointer(byteHandle, length);
    }
    public static void Free(ValueMutPtr handle) {
        Memory.Free(handle);
    }
}
testcase Given_memory_alloc_zeroed_length_When_executed_Then_memory_alloc_zeroed_length()
{
    let handle = MemoryTestHelpers.AllocZeroed(4usize, 1usize);
    let span = MemoryTestHelpers.SpanFrom(handle, 4usize);
    Assert.That(span.Length == 4usize).IsTrue();
    MemoryTestHelpers.Free(handle);
}
testcase Given_memory_alloc_zeroed_first_byte_zero_When_executed_Then_memory_alloc_zeroed_first_byte_zero()
{
    let handle = MemoryTestHelpers.AllocZeroed(4usize, 1usize);
    let span = MemoryTestHelpers.SpanFrom(handle, 4usize);
    Assert.That(span[0usize] == 0u8).IsTrue();
    MemoryTestHelpers.Free(handle);
}
testcase Given_memory_alloc_zeroed_last_byte_zero_When_executed_Then_memory_alloc_zeroed_last_byte_zero()
{
    let handle = MemoryTestHelpers.AllocZeroed(4usize, 1usize);
    let span = MemoryTestHelpers.SpanFrom(handle, 4usize);
    Assert.That(span[3usize] == 0u8).IsTrue();
    MemoryTestHelpers.Free(handle);
}
testcase Given_memory_set_updates_byte_When_executed_Then_memory_set_updates_byte()
{
    let handle = MemoryTestHelpers.AllocZeroed(4usize, 1usize);
    let span = MemoryTestHelpers.SpanFrom(handle, 4usize);
    Memory.Set(handle, 0x7Fu8, 4usize);
    Assert.That(span[1usize] == 0x7Fu8).IsTrue();
    MemoryTestHelpers.Free(handle);
}
testcase Given_memory_copy_copies_middle_byte_When_executed_Then_memory_copy_copies_middle_byte()
{
    let source = MemoryTestHelpers.Alloc(4usize, 1usize);
    let srcSpan = MemoryTestHelpers.SpanFrom(source, 4usize);
    srcSpan[0usize] = (byte) 1;
    srcSpan[1usize] = (byte) 2;
    srcSpan[2usize] = (byte) 3;
    srcSpan[3usize] = (byte) 4;
    let dest = MemoryTestHelpers.Alloc(4usize, 1usize);
    Memory.Copy(dest, source, 4usize);
    let destSpan = MemoryTestHelpers.SpanFrom(dest, 4usize);
    Assert.That(destSpan[2usize] == 3u8).IsTrue();
    MemoryTestHelpers.Free(source);
    MemoryTestHelpers.Free(dest);
}
testcase Given_memory_move_keeps_first_byte_When_executed_Then_memory_move_keeps_first_byte()
{
    let source = MemoryTestHelpers.Alloc(4usize, 1usize);
    let srcSpan = MemoryTestHelpers.SpanFrom(source, 4usize);
    srcSpan[0usize] = (byte) 1;
    srcSpan[1usize] = (byte) 2;
    srcSpan[2usize] = (byte) 3;
    srcSpan[3usize] = (byte) 4;
    let dest = MemoryTestHelpers.Alloc(4usize, 1usize);
    Memory.Copy(dest, source, 4usize);
    let destSpan = MemoryTestHelpers.SpanFrom(dest, 4usize);
    Memory.Move(dest, dest, 4usize);
    Assert.That(destSpan[0usize] == 1u8).IsTrue();
    MemoryTestHelpers.Free(source);
    MemoryTestHelpers.Free(dest);
}
testcase Given_memory_realloc_offset_updates_address_When_executed_Then_memory_realloc_offset_updates_address()
{
    let handle = MemoryTestHelpers.Alloc(1usize, 1usize);
    var resized = ValuePointer.NullMut(0usize, 0usize);
    let _ = Memory.Realloc(handle, 1usize, 2usize, 1usize, out resized);
    var baseAddr : nuint = 0;
    unsafe {
        baseAddr = Pointer.AddressOf(resized.Pointer);
    }
    let shifted = Memory.Offset(resized, 1isize);
    var shiftedAddr : nuint = 0;
    unsafe {
        shiftedAddr = Pointer.AddressOf(shifted.Pointer);
    }
    Assert.That(shiftedAddr == baseAddr + (nuint) 1).IsTrue();
    MemoryTestHelpers.Free(resized);
}
testcase Given_maybe_uninit_init_sets_initialized_When_executed_Then_maybe_uninit_init_sets_initialized()
{
    var slot = MaybeUninit <int >.Init(42);
    Assert.That(slot.IsInitialized()).IsTrue();
    slot.ForgetInit();
}
testcase Given_maybe_uninit_assume_init_read_returns_value_When_executed_Then_maybe_uninit_assume_init_read_returns_value()
{
    var slot = MaybeUninit <int >.Init(42);
    var value : int = slot.AssumeInitRead();
    Assert.That(value == 42).IsTrue();
    slot.ForgetInit();
}
testcase Given_maybe_uninit_assume_init_returns_value_When_executed_Then_maybe_uninit_assume_init_returns_value()
{
    var slot = MaybeUninit <int >.Init(42);
    var value : int = slot.AssumeInit();
    Assert.That(value == 42).IsTrue();
}
testcase Given_maybe_uninit_assume_init_clears_flag_When_executed_Then_maybe_uninit_assume_init_clears_flag()
{
    var slot = MaybeUninit <int >.Init(42);
    let _ = slot.AssumeInit();
    Assert.That(slot.IsInitialized()).IsFalse();
}
testcase Given_maybe_uninit_write_assume_value_When_executed_Then_maybe_uninit_write_assume_value()
{
    var slot = MaybeUninit <int >.Uninit();
    slot.Write(7);
    var value : int = slot.AssumeInitRead();
    Assert.That(value == 7).IsTrue();
    slot.ForgetInit();
}
testcase Given_maybe_uninit_forget_clears_flag_When_executed_Then_maybe_uninit_forget_clears_flag()
{
    var slot = MaybeUninit <int >.Uninit();
    slot.Write(7);
    slot.ForgetInit();
    Assert.That(slot.IsInitialized()).IsFalse();
}
testcase Given_maybe_uninit_create_zeroed_is_initialized_When_executed_Then_maybe_uninit_create_zeroed_is_initialized()
{
    var slot = MaybeUninit <int >.CreateZeroed();
    Assert.That(slot.IsInitialized()).IsTrue();
    slot.ForgetInit();
}
testcase Given_maybe_uninit_create_zeroed_value_zero_When_executed_Then_maybe_uninit_create_zeroed_value_zero()
{
    var slot = MaybeUninit <int >.CreateZeroed();
    var value : int = slot.AssumeInitRead();
    Assert.That(value == 0).IsTrue();
    slot.ForgetInit();
}
testcase Given_maybe_uninit_assume_init_throws_When_executed_Then_maybe_uninit_assume_init_throws()
{
    var slot = MaybeUninit <int >.Uninit();
    var threw = false;
    try {
        let _ = slot.AssumeInit();
    }
    catch(InvalidOperationException) {
        threw = true;
    }
    Assert.That(threw).IsTrue();
}
testcase Given_intrinsics_zero_init_raw_zeroes_first_byte_When_executed_Then_intrinsics_zero_init_raw_zeroes_first_byte()
{
    let handle = MemoryTestHelpers.Alloc(4usize, 1usize);
    let span = MemoryTestHelpers.SpanFrom(handle, 4usize);
    span[0usize] = (byte) 9;
    span[1usize] = (byte) 8;
    unsafe {
        Intrinsics.ZeroInitRaw(handle.Pointer, 4usize);
    }
    Assert.That(span[0usize] == 0u8).IsTrue();
    MemoryTestHelpers.Free(handle);
}
testcase Given_intrinsics_zero_init_raw_zeroes_second_byte_When_executed_Then_intrinsics_zero_init_raw_zeroes_second_byte()
{
    let handle = MemoryTestHelpers.Alloc(4usize, 1usize);
    let span = MemoryTestHelpers.SpanFrom(handle, 4usize);
    span[0usize] = (byte) 9;
    span[1usize] = (byte) 8;
    unsafe {
        Intrinsics.ZeroInitRaw(handle.Pointer, 4usize);
    }
    Assert.That(span[1usize] == 0u8).IsTrue();
    MemoryTestHelpers.Free(handle);
}
testcase Given_maybe_uninit_write_throws_when_initialized_When_executed_Then_maybe_uninit_write_throws_when_initialized()
{
    var slot = MaybeUninit <int >.Init(1);
    var threw = false;
    try {
        slot.Write(2);
    }
    catch(InvalidOperationException) {
        threw = true;
    }
    Assert.That(threw).IsTrue();
    slot.ForgetInit();
}
testcase Given_maybe_uninit_assume_init_read_throws_when_uninitialized_When_executed_Then_maybe_uninit_assume_init_read_throws_when_uninitialized()
{
    var slot = MaybeUninit <int >.Init(1);
    slot.ForgetInit();
    var threw = false;
    try {
        let _ = slot.AssumeInitRead();
    }
    catch(InvalidOperationException) {
        threw = true;
    }
    Assert.That(threw).IsTrue();
}
testcase Given_maybe_uninit_mark_initialized_defaults_to_zero_When_executed_Then_maybe_uninit_mark_initialized_defaults_to_zero()
{
    var slot = MaybeUninit <int >.Uninit();
    slot.MarkInitialized();
    var value : int = slot.AssumeInitRead();
    Assert.That(value == 0).IsTrue();
    slot.ForgetInit();
}
testcase Given_maybe_uninit_mark_initialized_forget_clears_flag_When_executed_Then_maybe_uninit_mark_initialized_forget_clears_flag()
{
    var slot = MaybeUninit <int >.Uninit();
    slot.MarkInitialized();
    slot.ForgetInit();
    Assert.That(slot.IsInitialized()).IsFalse();
}
testcase Given_maybe_uninit_assume_init_ref_reads_value_When_executed_Then_maybe_uninit_assume_init_ref_reads_value()
{
    var slot = MaybeUninit <int >.Uninit();
    slot.Write(11);
    let _ = slot.AssumeInitRef();
    var value : int = slot.AssumeInitRead();
    Assert.That(value == 11).IsTrue();
    slot.ForgetInit();
}
testcase Given_intrinsics_zero_init_raw_keeps_null_pointer_When_executed_Then_intrinsics_zero_init_raw_keeps_null_pointer()
{
    unsafe {
        let ptr = Pointer.NullMut <byte >();
        Intrinsics.ZeroInitRaw(ptr, 0usize);
        Intrinsics.ZeroInitRaw(ptr, 4usize);
        Assert.That(Pointer.IsNull(ptr)).IsTrue();
    }
}
testcase Given_memory_offset_null_returns_null_When_executed_Then_memory_offset_null_returns_null()
{
    let nullHandle = ValuePointer.NullMut(0usize, 0usize);
    let same = Memory.Offset(nullHandle, 8isize);
    Assert.That(ValuePointer.IsNullMut(same)).IsTrue();
}
testcase Given_memory_offset_zero_keeps_address_When_executed_Then_memory_offset_zero_keeps_address()
{
    let handle = MemoryTestHelpers.Alloc(2usize, 1usize);
    let unchanged = Memory.Offset(handle, 0isize);
    unsafe {
        Assert.That(Pointer.AddressOf(handle.Pointer) == Pointer.AddressOf(unchanged.Pointer)).IsTrue();
    }
    MemoryTestHelpers.Free(handle);
}
testcase Given_stackalloc_buffer_copies_first_byte_When_executed_Then_stackalloc_buffer_copies_first_byte()
{
    var span = StackAlloc.Span <byte >(4usize);
    span[0usize] = 1u8;
    span[3usize] = 4u8;
    let readOnlySpan = span.AsReadOnly();
    let buffer = StackAlloc.FromSpan <byte >(readOnlySpan);
    unsafe {
        let bytes = ReadOnlySpan <byte >.FromValuePointer(buffer, 4usize);
        var first : byte = bytes[0usize];
        Assert.That(first == 1u8).IsTrue();
    }
}
testcase Given_stackalloc_buffer_copies_last_byte_When_executed_Then_stackalloc_buffer_copies_last_byte()
{
    var span = StackAlloc.Span <byte >(4usize);
    span[0usize] = 1u8;
    span[3usize] = 4u8;
    let readOnlySpan = span.AsReadOnly();
    let buffer = StackAlloc.FromSpan <byte >(readOnlySpan);
    unsafe {
        let bytes = ReadOnlySpan <byte >.FromValuePointer(buffer, 4usize);
        var last : byte = bytes[3usize];
        Assert.That(last == 4u8).IsTrue();
    }
}
testcase Given_stackalloc_from_span_returns_non_null_When_executed_Then_stackalloc_from_span_returns_non_null()
{
    var span = StackAlloc.Span <byte >(4usize);
    let fromSpan = StackAlloc.FromSpan <byte >(span);
    Assert.That(ValuePointer.IsNullMut(fromSpan)).IsFalse();
}
testcase Given_memory_intrinsics_zero_value_default_When_executed_Then_memory_intrinsics_zero_value_default()
{
    var value : int = Intrinsics.ZeroValue <int >();
    Assert.That(value == 0).IsTrue();
}
testcase Given_memory_intrinsics_zero_init_sets_zero_When_executed_Then_memory_intrinsics_zero_init_sets_zero()
{
    var target : int = 0;
    Intrinsics.ZeroInit(out target);
    Assert.That(target == 0).IsTrue();
}
