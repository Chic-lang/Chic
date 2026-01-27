namespace Std.Memory;
import Std.Core;
import Std.Numeric;
import Std.Runtime.Collections;
import Std.Core.Testing;
/// <summary>Page-locked host allocation suitable for deterministic DMA.</summary>
public struct Pinned
{
    public uint Length;
}
/// <summary>Unified host/device allocation when supported by the backend.</summary>
public struct Unified
{
    public uint Length;
}
public enum MemoryErrorCode
{
    BorrowActive, Freed,
}
public class MemoryError : Exception
{
    public MemoryErrorCode Code;
    public init() : base() {
        Code = MemoryErrorCode.BorrowActive;
    }
}
/// <summary>Pinned buffer that enforces single mutable borrow.</summary>
public class PinnedBuffer
{
    private * mut @expose_address byte _ptr;
    private usize _alloc_size;
    private usize _alloc_align;
    private uint _length;
    private bool _borrowed;
    private bool _freed;
    public init(uint length) {
        var allocation = ValuePointer.NullMut(0usize, 0usize);
        let status = Memory.Alloc(NumericUnchecked.ToUSize(length), 1usize, out allocation);
        if (status != AllocationError.Success)
        {
            throw new InvalidOperationException("pinned allocation failed");
        }
        _ptr = allocation.Pointer;
        _alloc_size = allocation.Size;
        _alloc_align = allocation.Alignment;
        _length = length;
        _borrowed = false;
        _freed = false;
    }
    public uint Length {
        get {
            return _length;
        }
    }
    public PinnedBorrowGuard BorrowMut() {
        if (_freed)
        {
            var err = new MemoryError();
            err.Code = MemoryErrorCode.Freed;
            err.Message = "pinned buffer is freed";
            throw err;
        }
        if (_borrowed)
        {
            var err = new MemoryError();
            err.Code = MemoryErrorCode.BorrowActive;
            err.Message = "pinned buffer already borrowed; wait for completion";
            throw err;
        }
        _borrowed = true;
        return new PinnedBorrowGuard(this);
    }
    public void Free() {
        if (_freed)
        {
            return;
        }
        if (_borrowed)
        {
            var err = new MemoryError();
            err.Code = MemoryErrorCode.BorrowActive;
            err.Message = "pinned buffer is borrowed; wait for event before free";
            throw err;
        }
        _freed = true;
        var handle = ValuePointer.NullMut(_alloc_size, _alloc_align);
        handle.Pointer = _ptr;
        handle.Size = _alloc_size;
        handle.Alignment = _alloc_align;
        Memory.Free(handle);
        unsafe {
            _ptr = Std.Numeric.Pointer.NullMut<byte>();
        }
        _alloc_size = 0usize;
        _alloc_align = 0usize;
        _length = 0u;
    }
    public Span <byte >AsMutSpan() {
        unsafe {
            let handle = ValuePointer.CreateMut(_ptr, 1usize, 1usize);
            return Span <byte >.FromValuePointer(handle, NumericUnchecked.ToUSize(_length));
        }
    }
    public void ReleaseBorrow() {
        _borrowed = false;
    }
    /// <summary>Testing hook to force borrow flag.</summary>
    public void SetBorrowedForTest(bool borrowed) {
        _borrowed = borrowed;
    }
}
public struct PinnedBorrowGuard
{
    private PinnedBuffer _buffer;
    public init(PinnedBuffer buffer) {
        _buffer = buffer;
    }
    public Span <byte >AsSpan() {
        return _buffer.AsMutSpan();
    }
    public void Release() {
        if (_buffer != null)
        {
            _buffer.ReleaseBorrow();
        }
    }
    public void dispose(ref this) {
        Release();
    }
}
/// <summary>Unified buffer with borrow tracking.</summary>
public class UnifiedBuffer
{
    private * mut @expose_address byte _ptr;
    private usize _alloc_size;
    private usize _alloc_align;
    private uint _length;
    private bool _borrowed;
    private bool _freed;
    public init(uint length) {
        var allocation = ValuePointer.NullMut(0usize, 0usize);
        let status = Memory.Alloc(NumericUnchecked.ToUSize(length), 1usize, out allocation);
        if (status != AllocationError.Success)
        {
            throw new InvalidOperationException("unified allocation failed");
        }
        _ptr = allocation.Pointer;
        _alloc_size = allocation.Size;
        _alloc_align = allocation.Alignment;
        _length = length;
        _borrowed = false;
        _freed = false;
    }
    public uint Length {
        get {
            return _length;
        }
    }
    public UnifiedBorrowGuard BorrowMut() {
        if (_freed)
        {
            var err = new MemoryError();
            err.Code = MemoryErrorCode.Freed;
            err.Message = "unified buffer is freed";
            throw err;
        }
        if (_borrowed)
        {
            var err = new MemoryError();
            err.Code = MemoryErrorCode.BorrowActive;
            err.Message = "unified buffer already borrowed; wait for completion";
            throw err;
        }
        _borrowed = true;
        return new UnifiedBorrowGuard(this);
    }
    public void Free() {
        if (_freed)
        {
            return;
        }
        if (_borrowed)
        {
            var err = new MemoryError();
            err.Code = MemoryErrorCode.BorrowActive;
            err.Message = "unified buffer is borrowed; wait for event before free";
            throw err;
        }
        _freed = true;
        var handle = ValuePointer.NullMut(_alloc_size, _alloc_align);
        handle.Pointer = _ptr;
        handle.Size = _alloc_size;
        handle.Alignment = _alloc_align;
        Memory.Free(handle);
        unsafe {
            _ptr = Std.Numeric.Pointer.NullMut<byte>();
        }
        _alloc_size = 0usize;
        _alloc_align = 0usize;
        _length = 0u;
    }
    public Span <byte >AsMutSpan() {
        unsafe {
            let handle = ValuePointer.CreateMut(_ptr, 1usize, 1usize);
            return Span <byte >.FromValuePointer(handle, NumericUnchecked.ToUSize(_length));
        }
    }
    public void ReleaseBorrow() {
        _borrowed = false;
    }
    public void SetBorrowedForTest(bool borrowed) {
        _borrowed = borrowed;
    }
}
public struct UnifiedBorrowGuard
{
    private UnifiedBuffer _buffer;
    public init(UnifiedBuffer buffer) {
        _buffer = buffer;
    }
    public Span <byte >AsSpan() {
        return _buffer.AsMutSpan();
    }
    public void Release() {
        if (_buffer != null)
        {
            _buffer.ReleaseBorrow();
        }
    }
    public void dispose(ref this) {
        Release();
    }
}
public static class PinnedAllocator
{
    public static Pinned AllocPinned(uint length) {
        var buffer = new PinnedBuffer(length);
        var pinned = CoreIntrinsics.DefaultValue <Pinned >();
        pinned.Length = buffer.Length;
        return pinned;
    }
    public static Unified AllocUnified(uint length) {
        var buffer = new UnifiedBuffer(length);
        var unified = CoreIntrinsics.DefaultValue <Unified >();
        unified.Length = buffer.Length;
        return unified;
    }
}
testcase Given_pinned_buffer_enforces_borrow_When_executed_Then_pinned_buffer_enforces_borrow()
{
    var buf = new PinnedBuffer(16u);
    var guard = buf.BorrowMut();
    guard.AsSpan()[0usize] = 1u8;
    var threw = false;
    try {
        let _ = buf.BorrowMut();
    }
    catch(MemoryError) {
        threw = true;
    }
    guard.Release();
    Assert.That(threw).IsTrue();
    let ok = buf.BorrowMut();
    ok.Release();
}
testcase Given_pinned_buffer_free_requires_no_borrow_When_executed_Then_pinned_buffer_free_requires_no_borrow()
{
    var buf = new PinnedBuffer(8u);
    buf.SetBorrowedForTest(true);
    var threw = false;
    try {
        buf.Free();
    }
    catch(MemoryError) {
        threw = true;
    }
    Assert.That(threw).IsTrue();
    buf.SetBorrowedForTest(false);
    buf.Free();
}
testcase Given_unified_buffer_rejects_after_free_When_executed_Then_unified_buffer_rejects_after_free()
{
    var buf = new UnifiedBuffer(4u);
    var guard = buf.BorrowMut();
    var span = guard.AsSpan();
    var idx = 0usize;
    while (idx <span.Length)
    {
        span[idx] = 7u8;
        idx = idx + 1usize;
    }
    guard.Release();
    buf.Free();
    var threw = false;
    try {
        let _ = buf.BorrowMut();
    }
    catch(MemoryError) {
        threw = true;
    }
    Assert.That(threw).IsTrue();
}
