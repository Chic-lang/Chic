namespace Std.Sync;
import Std;
import Std.Numeric;
import Std.Memory;
import Std.Runtime;
import Std.Runtime.Collections;
import Std.Runtime.InteropServices;
@repr(c) public enum MemoryOrder
{
    Relaxed, Acquire, Release, AcqRel, SeqCst,
}
private static byte ToOrderByte(MemoryOrder order) {
    unchecked {
        return(byte) order;
    }
}
// Atomic wrappers that map to runtime intrinsics when available. Ordering hints
// follow the MemoryOrder enum and are forwarded to the runtime implementation.
@repr(c) public struct AtomicBool
{
    private bool _value;
    public init(bool value = false) {
        _value = value;
    }
    public bool Load(MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsConstPtr <_AtomicBoolRepr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_bool_load(ptr, ToOrderByte(order)) != 0;
        }
    }
    public void Store(bool value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicBoolRepr >(& _value);
            let byteValue = Std.Numeric.NumericUnchecked.ToByte(value ?1 : 0);
            RuntimeIntrinsics.chic_rt_atomic_bool_store(ptr, byteValue, ToOrderByte(order));
        }
    }
    public bool CompareExchange(bool expected, bool desired, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicBoolRepr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_bool_compare_exchange(ptr, Std.Numeric.NumericUnchecked.ToByte(expected ?1 : 0),
            Std.Numeric.NumericUnchecked.ToByte(desired ?1 : 0), ToOrderByte(order)) != 0;
        }
    }
}
@repr(c) public struct AtomicUsize
{
    private usize _value;
    public init(usize value = 0) {
        _value = value;
    }
    public usize Load(MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsConstPtr <_AtomicUsizeRepr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_usize_load(ptr, ToOrderByte(order));
        }
    }
    public void Store(usize value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicUsizeRepr >(& _value);
            RuntimeIntrinsics.chic_rt_atomic_usize_store(ptr, value, ToOrderByte(order));
        }
    }
    public usize FetchAdd(usize value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicUsizeRepr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_usize_fetch_add(ptr, value, ToOrderByte(order));
        }
    }
    public usize FetchSub(usize value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicUsizeRepr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_usize_fetch_sub(ptr, value, ToOrderByte(order));
        }
    }
}
@repr(c) public struct AtomicI32
{
    private int _value;
    public init(int value = 0) {
        _value = value;
    }
    public int Load(MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsConstPtr <_AtomicI32Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_i32_load(ptr, ToOrderByte(order));
        }
    }
    public void Store(int value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicI32Repr >(& _value);
            RuntimeIntrinsics.chic_rt_atomic_i32_store(ptr, value, ToOrderByte(order));
        }
    }
    public int FetchAdd(int value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicI32Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_i32_fetch_add(ptr, value, ToOrderByte(order));
        }
    }
    public int FetchSub(int value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicI32Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_i32_fetch_sub(ptr, value, ToOrderByte(order));
        }
    }
    public bool CompareExchange(int expected, int desired, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicI32Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_i32_compare_exchange(ptr, expected, desired, ToOrderByte(order)) != 0;
        }
    }
}
@repr(c) public struct AtomicU32
{
    private uint _value;
    public init(uint value = 0u) {
        _value = value;
    }
    public uint Load(MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsConstPtr <_AtomicU32Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_u32_load(ptr, ToOrderByte(order));
        }
    }
    public void Store(uint value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicU32Repr >(& _value);
            RuntimeIntrinsics.chic_rt_atomic_u32_store(ptr, value, ToOrderByte(order));
        }
    }
    public uint FetchAdd(uint value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicU32Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_u32_fetch_add(ptr, value, ToOrderByte(order));
        }
    }
    public uint FetchSub(uint value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicU32Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_u32_fetch_sub(ptr, value, ToOrderByte(order));
        }
    }
    public bool CompareExchange(uint expected, uint desired, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicU32Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_u32_compare_exchange(ptr, expected, desired, ToOrderByte(order)) != 0;
        }
    }
}
@repr(c) public struct AtomicI64
{
    private long _value;
    public init(long value = 0L) {
        _value = value;
    }
    public long Load(MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsConstPtr <_AtomicI64Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_i64_load(ptr, ToOrderByte(order));
        }
    }
    public void Store(long value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicI64Repr >(& _value);
            RuntimeIntrinsics.chic_rt_atomic_i64_store(ptr, value, ToOrderByte(order));
        }
    }
    public long FetchAdd(long value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicI64Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_i64_fetch_add(ptr, value, ToOrderByte(order));
        }
    }
    public long FetchSub(long value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicI64Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_i64_fetch_sub(ptr, value, ToOrderByte(order));
        }
    }
    public bool CompareExchange(long expected, long desired, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicI64Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_i64_compare_exchange(ptr, expected, desired, ToOrderByte(order)) != 0;
        }
    }
}
@repr(c) public struct AtomicU64
{
    private ulong _value;
    public init(ulong value = 0UL) {
        _value = value;
    }
    public ulong Load(MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsConstPtr <_AtomicU64Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_u64_load(ptr, ToOrderByte(order));
        }
    }
    public void Store(ulong value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicU64Repr >(& _value);
            RuntimeIntrinsics.chic_rt_atomic_u64_store(ptr, value, ToOrderByte(order));
        }
    }
    public ulong FetchAdd(ulong value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicU64Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_u64_fetch_add(ptr, value, ToOrderByte(order));
        }
    }
    public ulong FetchSub(ulong value, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicU64Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_u64_fetch_sub(ptr, value, ToOrderByte(order));
        }
    }
    public bool CompareExchange(ulong expected, ulong desired, MemoryOrder order = MemoryOrder.SeqCst) {
        unsafe {
            let ptr = PointerIntrinsics.AsMutPtr <_AtomicU64Repr >(& _value);
            return RuntimeIntrinsics.chic_rt_atomic_u64_compare_exchange(ptr, expected, desired, ToOrderByte(order)) != 0;
        }
    }
}
@repr(c) internal struct _AtomicBoolRepr
{
    public byte Value;
}
@repr(c) internal struct _AtomicUsizeRepr
{
    public usize Value;
}
@repr(c) internal struct _AtomicI32Repr
{
    public int Value;
}
@repr(c) internal struct _AtomicU32Repr
{
    public uint Value;
}
@repr(c) internal struct _AtomicI64Repr
{
    public long Value;
}
@repr(c) internal struct _AtomicU64Repr
{
    public ulong Value;
}
@Intrinsic @StructLayout(LayoutKind.Sequential) @thread_safe @shareable public struct Lock
{
    private usize _handle;
    public init() {
        _handle = RuntimeIntrinsics.chic_rt_lock_create();
        if (_handle == 0usize)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("failed to create lock"));
        }
    }
    public LockGuard Enter() throws InvalidOperationException {
        EnsureHandle();
        RuntimeIntrinsics.chic_rt_lock_enter(_handle);
        return LockGuard.Create(_handle, true);
    }
    public bool TryEnter(out LockGuard guard) throws InvalidOperationException {
        EnsureHandle();
        let acquired = RuntimeIntrinsics.chic_rt_lock_try_enter(_handle);
        guard = LockGuard.Create(_handle, acquired);
        return acquired;
    }
    public void EnterRaw() throws InvalidOperationException {
        EnsureHandle();
        RuntimeIntrinsics.chic_rt_lock_enter(_handle);
    }
    public bool TryEnterRaw() throws InvalidOperationException {
        EnsureHandle();
        return RuntimeIntrinsics.chic_rt_lock_try_enter(_handle);
    }
    public void ExitRaw() throws InvalidOperationException {
        EnsureHandle();
        RuntimeIntrinsics.chic_rt_lock_exit(_handle);
    }
    public bool IsHeld {
        get {
            return _handle != 0usize && RuntimeIntrinsics.chic_rt_lock_is_held(_handle);
        }
    }
    public bool IsHeldByCurrentThread {
        get {
            return _handle != 0usize && RuntimeIntrinsics.chic_rt_lock_is_held_by_current_thread(_handle);
        }
    }
    public void dispose(ref this) {
        if (_handle != 0usize)
        {
            RuntimeIntrinsics.chic_rt_lock_destroy(_handle);
            _handle = 0usize;
        }
    }
    private void EnsureHandle() throws InvalidOperationException {
        if (_handle == 0usize)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("lock is not initialised"));
        }
    }
}
@StructLayout(LayoutKind.Sequential) @not_shareable public struct LockGuard
{
    private usize _handle;
    private bool _held;
    internal static LockGuard Create(usize handle, bool held) {
        var guard = CoreIntrinsics.DefaultValue <LockGuard >();
        guard._handle = handle;
        guard._held = held;
        return guard;
    }
    public bool Held => _held;
    public void Release(ref this) throws InvalidOperationException {
        dispose();
    }
    public void Dispose(ref this) throws InvalidOperationException {
        dispose();
    }
    public void dispose(ref this) throws InvalidOperationException {
        if (_held && _handle != 0usize)
        {
            RuntimeIntrinsics.chic_rt_lock_exit(_handle);
            _held = false;
            _handle = 0usize;
        }
    }
}
public sealed class Mutex <T >
{
    private usize _handle;
    private Std.Memory.MaybeUninit <T >_slot;
    private bool _initialized;
    public init() : self(Std.Memory.Intrinsics.ZeroValue <T >()) {
    }
    public init(T value) {
        _slot = Std.Memory.MaybeUninit <T >.Init(value);
        _initialized = true;
        _handle = RuntimeIntrinsics.chic_rt_mutex_create();
        if (_handle == 0usize)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("failed to create mutex"));
        }
    }
    public usize Handle => _handle;
    internal ref T BorrowValue(ref this) throws InvalidOperationException {
        if (! _initialized)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("mutex value is not initialised"));
        }
        return _slot.AssumeInitRef();
    }
    internal void SetValue(T value) {
        if (_initialized)
        {
            var slot = _slot;
            slot.dispose();
            _slot = slot;
        }
        _slot = Std.Memory.MaybeUninit <T >.Init(value);
        _initialized = true;
    }
    public MutexGuard <T >Lock() throws InvalidOperationException {
        EnsureHandle();
        RuntimeIntrinsics.chic_rt_mutex_lock(_handle);
        return MutexGuard <T >.Create(this, true);
    }
    public bool TryLock(out MutexGuard <T >guard) throws InvalidOperationException {
        EnsureHandle();
        let acquired = RuntimeIntrinsics.chic_rt_mutex_try_lock(_handle);
        guard = MutexGuard <T >.Create(this, acquired);
        return acquired;
    }
    internal void Unlock() throws InvalidOperationException {
        EnsureHandle();
        RuntimeIntrinsics.chic_rt_mutex_unlock(_handle);
    }
    public void dispose(ref this) throws InvalidOperationException {
        if (_handle != 0usize)
        {
            RuntimeIntrinsics.chic_rt_mutex_destroy(_handle);
            _handle = 0usize;
        }
        if (_initialized)
        {
            var slot = _slot;
            slot.dispose();
            _slot = slot;
            _initialized = false;
        }
    }
    private void EnsureHandle() throws InvalidOperationException {
        if (_handle == 0usize)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("mutex is not initialised"));
        }
    }
}
public struct MutexGuard <T >
{
    private Mutex <T >_owner;
    private bool _held;
    internal static MutexGuard <T >Create(Mutex <T >owner, bool held) {
        var guard = CoreIntrinsics.DefaultValue <MutexGuard <T >> ();
        guard._owner = owner;
        guard._held = held;
        return guard;
    }
    public bool Held => _held;
    public ref T Value {
        get {
            if (! _held || _owner == null)
            {
                throw new InvalidOperationException(StringRuntime.FromStr("mutex guard not held"));
            }
            var owner = _owner;
            return owner.BorrowValue();
        }
    }
    public void ReplaceValue(ref var this, T value) throws InvalidOperationException {
        if (! _held || _owner == null)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("mutex guard not held"));
        }
        _owner.SetValue(value);
    }
    public void Release(ref var this) throws InvalidOperationException {
        if (_held && _owner != null)
        {
            _owner.Unlock();
            _held = false;
        }
    }
    public void dispose(ref var this) throws InvalidOperationException {
        if (_held && _owner != null)
        {
            _owner.Unlock();
            _held = false;
        }
    }
}
public sealed class RwLock <T >
{
    private usize _handle;
    private Std.Memory.MaybeUninit <T >_slot;
    private bool _initialized;
    public init() : self(Std.Memory.Intrinsics.ZeroValue <T >()) {
    }
    public init(T value) {
        _slot = Std.Memory.MaybeUninit <T >.Init(value);
        _initialized = true;
        _handle = RuntimeIntrinsics.chic_rt_rwlock_create();
        if (_handle == 0usize)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("failed to create rwlock"));
        }
    }
    internal usize Handle => _handle;
    internal ref T BorrowValue(ref this) throws InvalidOperationException {
        if (! _initialized)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("rwlock value is not initialised"));
        }
        return _slot.AssumeInitRef();
    }
    public RwLockReadGuard <T >Read() throws InvalidOperationException {
        EnsureHandle();
        RuntimeIntrinsics.chic_rt_rwlock_read_lock(_handle);
        return new RwLockReadGuard <T >(this, true);
    }
    public bool TryRead(out RwLockReadGuard <T >guard) throws InvalidOperationException {
        EnsureHandle();
        let acquired = RuntimeIntrinsics.chic_rt_rwlock_try_read_lock(_handle);
        guard = new RwLockReadGuard <T >(this, acquired);
        return acquired;
    }
    public RwLockWriteGuard <T >Write() throws InvalidOperationException {
        EnsureHandle();
        RuntimeIntrinsics.chic_rt_rwlock_write_lock(_handle);
        return new RwLockWriteGuard <T >(this, true);
    }
    public bool TryWrite(out RwLockWriteGuard <T >guard) throws InvalidOperationException {
        EnsureHandle();
        let acquired = RuntimeIntrinsics.chic_rt_rwlock_try_write_lock(_handle);
        guard = new RwLockWriteGuard <T >(this, acquired);
        return acquired;
    }
    internal void ReadUnlock() throws InvalidOperationException {
        EnsureHandle();
        RuntimeIntrinsics.chic_rt_rwlock_read_unlock(_handle);
    }
    internal void WriteUnlock() throws InvalidOperationException {
        EnsureHandle();
        RuntimeIntrinsics.chic_rt_rwlock_write_unlock(_handle);
    }
    public void dispose(ref this) throws InvalidOperationException {
        if (_handle != 0usize)
        {
            RuntimeIntrinsics.chic_rt_rwlock_destroy(_handle);
            _handle = 0usize;
        }
        if (_initialized)
        {
            var slot = _slot;
            slot.dispose();
            _slot = slot;
            _initialized = false;
        }
    }
    private void EnsureHandle() throws InvalidOperationException {
        if (_handle == 0usize)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("rwlock is not initialised"));
        }
    }
}
public struct RwLockReadGuard <T >
{
    private RwLock <T >_owner;
    private bool _held;
    internal init(RwLock <T >owner, bool held) {
        _owner = owner;
        _held = held;
    }
    internal static RwLockReadGuard <T >Create(RwLock <T >owner, bool held) {
        return new RwLockReadGuard <T >(owner, held);
    }
    public ref readonly T Value {
        get {
            if (! _held || _owner == null)
            {
                throw new InvalidOperationException(StringRuntime.FromStr("rwlock read guard not held"));
            }
            var owner = _owner;
            return owner.BorrowValue();
        }
    }
    public void Release(ref var this) throws InvalidOperationException {
        if (_held && _owner != null)
        {
            _owner.ReadUnlock();
            _held = false;
        }
    }
    public void dispose(ref var this) throws InvalidOperationException {
        if (_held && _owner != null)
        {
            _owner.ReadUnlock();
            _held = false;
        }
    }
}
public struct RwLockWriteGuard <T >
{
    private RwLock <T >_owner;
    private bool _held;
    internal init(RwLock <T >owner, bool held) {
        _owner = owner;
        _held = held;
    }
    internal static RwLockWriteGuard <T >Create(RwLock <T >owner, bool held) {
        return new RwLockWriteGuard <T >(owner, held);
    }
    public ref T Value {
        get {
            if (! _held || _owner == null)
            {
                throw new InvalidOperationException(StringRuntime.FromStr("rwlock write guard not held"));
            }
            var owner = _owner;
            return owner.BorrowValue();
        }
    }
    public void Release(ref var this) throws InvalidOperationException {
        if (_held && _owner != null)
        {
            _owner.WriteUnlock();
            _held = false;
        }
    }
    public void dispose(ref var this) throws InvalidOperationException {
        if (_held && _owner != null)
        {
            _owner.WriteUnlock();
            _held = false;
        }
    }
}
public sealed class CondVar
{
    private usize _handle;
    public init() {
        _handle = RuntimeIntrinsics.chic_rt_condvar_create();
        if (_handle == 0usize)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("failed to create condition variable"));
        }
    }
    public void NotifyOne() throws InvalidOperationException {
        EnsureHandle();
        RuntimeIntrinsics.chic_rt_condvar_notify_one(_handle);
    }
    public void NotifyAll() throws InvalidOperationException {
        EnsureHandle();
        RuntimeIntrinsics.chic_rt_condvar_notify_all(_handle);
    }
    public void Wait <T >(ref Mutex <T >mutex, ref MutexGuard <T >guard) throws InvalidOperationException {
        EnsureHandle();
        if (! guard.Held)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("mutex guard not held for wait"));
        }
        RuntimeIntrinsics.chic_rt_condvar_wait(_handle, mutex.Handle);
    }
    public void dispose(ref this) throws InvalidOperationException {
        if (_handle != 0usize)
        {
            RuntimeIntrinsics.chic_rt_condvar_destroy(_handle);
            _handle = 0usize;
        }
    }
    private void EnsureHandle() throws InvalidOperationException {
        if (_handle == 0usize)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("condition variable is not initialised"));
        }
    }
}
public interface OnceCallback
{
    void Invoke();
}
// Internal stub provides a vtable for codegen so OnceCallback trait calls have metadata even
// when no user implementations are present in the compilation unit.
internal sealed class __OnceCallbackStub : OnceCallback
{
    public void Invoke() {
    }
}
public sealed class Once
{
    private usize _handle;
    public init() {
        _handle = RuntimeIntrinsics.chic_rt_once_create();
        if (_handle == 0usize)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("failed to create once handle"));
        }
    }
    public bool Call(OnceCallback callback) throws InvalidOperationException {
        if (callback == null)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("once callback must not be null"));
        }
        if (TryBegin ())
        {
            callback.Invoke();
            Complete();
            return true;
        }
        Wait();
        return false;
    }
    public bool TryBegin() throws InvalidOperationException {
        EnsureHandle();
        return RuntimeIntrinsics.chic_rt_once_try_begin(_handle);
    }
    public void Complete() throws InvalidOperationException {
        EnsureHandle();
        RuntimeIntrinsics.chic_rt_once_complete(_handle);
    }
    public void Wait() throws InvalidOperationException {
        EnsureHandle();
        RuntimeIntrinsics.chic_rt_once_wait(_handle);
    }
    public bool IsCompleted {
        get {
            EnsureHandle();
            return RuntimeIntrinsics.chic_rt_once_is_completed(_handle);
        }
    }
    public void dispose(ref this) throws InvalidOperationException {
        if (_handle != 0usize)
        {
            RuntimeIntrinsics.chic_rt_once_destroy(_handle);
            _handle = 0usize;
        }
    }
    private void EnsureHandle() throws InvalidOperationException {
        if (_handle == 0usize)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("once handle is not initialised"));
        }
    }
}
@repr(c) public struct __StdSyncArcHandle
{
    public * mut @expose_address byte Pointer;
}
@repr(c) public struct __StdSyncWeakHandle
{
    public * mut @expose_address byte Pointer;
}
@repr(c) public struct __StdSyncRcHandle
{
    public * mut @expose_address byte Pointer;
}
@repr(c) public struct __StdSyncWeakRcHandle
{
    public * mut @expose_address byte Pointer;
}
public static class __StdSyncPointerHelpers
{
    public static * mut @expose_address byte NullMut() {
        let handle = ValuePointer.NullMut(0usize, 0usize);
        return handle.Pointer;
    }
    public static * const @readonly @expose_address byte NullConst() {
        let handle = ValuePointer.NullConst(0usize, 0usize);
        return handle.Pointer;
    }
    public static bool IsNull(* mut @expose_address byte pointer) {
        unsafe {
            let handle = ValuePointer.CreateMut(PointerIntrinsics.AsByteMut(pointer), 0usize, 0usize);
            return ValuePointer.IsNullMut(handle);
        }
    }
    public static bool IsNullConst(* const @readonly @expose_address byte pointer) {
        unsafe {
            let handle = ValuePointer.CreateConst(PointerIntrinsics.AsByteConst(pointer), 0usize, 0usize);
            return ValuePointer.IsNullConst(handle);
        }
    }
}
public static class RuntimeIntrinsics
{
    @extern("C") public static extern int chic_rt_arc_new(* mut @expose_address __StdSyncArcHandle dest, * const @readonly @expose_address byte src,
    usize size, usize align, usize drop_fn, u64 type_id);
    @extern("C") public static extern int chic_rt_arc_clone(* mut @expose_address __StdSyncArcHandle dest, * const @readonly @expose_address __StdSyncArcHandle src);
    @extern("C") public static extern void chic_rt_arc_drop(* mut @expose_address __StdSyncArcHandle target);
    @extern("C") public static extern * const @readonly @expose_address byte chic_rt_arc_get(* const @readonly @expose_address __StdSyncArcHandle src);
    @extern("C") public static extern * mut @expose_address byte chic_rt_arc_get_mut(* mut @expose_address __StdSyncArcHandle src);
    @extern("C") public static extern int chic_rt_arc_downgrade(* mut @expose_address __StdSyncWeakHandle dest, * const @readonly @expose_address __StdSyncArcHandle src);
    @extern("C") public static extern int chic_rt_weak_clone(* mut @expose_address __StdSyncWeakHandle dest, * const @readonly @expose_address __StdSyncWeakHandle src);
    @extern("C") public static extern void chic_rt_weak_drop(* mut @expose_address __StdSyncWeakHandle target);
    @extern("C") public static extern int chic_rt_weak_upgrade(* mut @expose_address __StdSyncArcHandle dest, * const @readonly @expose_address __StdSyncWeakHandle src);
    @extern("C") public static extern int chic_rt_rc_new(* mut @expose_address __StdSyncRcHandle dest, * const @readonly @expose_address byte src,
    usize size, usize align, usize drop_fn, u64 type_id);
    @extern("C") public static extern int chic_rt_rc_clone(* mut @expose_address __StdSyncRcHandle dest, * const @readonly @expose_address __StdSyncRcHandle src);
    @extern("C") public static extern void chic_rt_rc_drop(* mut @expose_address __StdSyncRcHandle target);
    @extern("C") public static extern * const @readonly @expose_address byte chic_rt_rc_get(* const @readonly @expose_address __StdSyncRcHandle src);
    @extern("C") public static extern * mut @expose_address byte chic_rt_rc_get_mut(* mut @expose_address __StdSyncRcHandle src);
    @extern("C") public static extern int chic_rt_rc_downgrade(* mut @expose_address __StdSyncWeakRcHandle dest, * const @readonly @expose_address __StdSyncRcHandle src);
    @extern("C") public static extern usize chic_rt_rc_strong_count(* const @readonly @expose_address __StdSyncRcHandle src);
    @extern("C") public static extern usize chic_rt_rc_weak_count(* const @readonly @expose_address __StdSyncRcHandle src);
    @extern("C") internal static extern int chic_rt_weak_rc_clone(* mut @expose_address __StdSyncWeakRcHandle dest,
    * const @readonly @expose_address __StdSyncWeakRcHandle src);
    @extern("C") internal static extern void chic_rt_weak_rc_drop(* mut @expose_address __StdSyncWeakRcHandle target);
    @extern("C") internal static extern int chic_rt_weak_rc_upgrade(* mut @expose_address __StdSyncRcHandle dest, * const @readonly @expose_address __StdSyncWeakRcHandle src);
    @extern("C") internal static extern usize chic_rt_mutex_create();
    @extern("C") internal static extern void chic_rt_mutex_destroy(usize handle);
    @extern("C") internal static extern void chic_rt_mutex_lock(usize handle);
    @extern("C") internal static extern bool chic_rt_mutex_try_lock(usize handle);
    @extern("C") internal static extern void chic_rt_mutex_unlock(usize handle);
    @extern("C") internal static extern usize chic_rt_lock_create();
    @extern("C") internal static extern void chic_rt_lock_destroy(usize handle);
    @extern("C") internal static extern void chic_rt_lock_enter(usize handle);
    @extern("C") internal static extern bool chic_rt_lock_try_enter(usize handle);
    @extern("C") internal static extern void chic_rt_lock_exit(usize handle);
    @extern("C") internal static extern bool chic_rt_lock_is_held(usize handle);
    @extern("C") internal static extern bool chic_rt_lock_is_held_by_current_thread(usize handle);
    @extern("C") internal static extern usize chic_rt_rwlock_create();
    @extern("C") internal static extern void chic_rt_rwlock_destroy(usize handle);
    @extern("C") internal static extern void chic_rt_rwlock_read_lock(usize handle);
    @extern("C") internal static extern bool chic_rt_rwlock_try_read_lock(usize handle);
    @extern("C") internal static extern void chic_rt_rwlock_read_unlock(usize handle);
    @extern("C") internal static extern void chic_rt_rwlock_write_lock(usize handle);
    @extern("C") internal static extern bool chic_rt_rwlock_try_write_lock(usize handle);
    @extern("C") internal static extern void chic_rt_rwlock_write_unlock(usize handle);
    @extern("C") internal static extern usize chic_rt_condvar_create();
    @extern("C") internal static extern void chic_rt_condvar_destroy(usize handle);
    @extern("C") internal static extern void chic_rt_condvar_notify_one(usize handle);
    @extern("C") internal static extern void chic_rt_condvar_notify_all(usize handle);
    @extern("C") internal static extern void chic_rt_condvar_wait(usize condvar_handle, usize mutex_handle);
    @extern("C") internal static extern usize chic_rt_once_create();
    @extern("C") internal static extern void chic_rt_once_destroy(usize handle);
    @extern("C") internal static extern bool chic_rt_once_try_begin(usize handle);
    @extern("C") internal static extern void chic_rt_once_complete(usize handle);
    @extern("C") internal static extern void chic_rt_once_wait(usize handle);
    @extern("C") internal static extern bool chic_rt_once_is_completed(usize handle);
    @extern("C") internal static extern byte chic_rt_atomic_bool_load(* const @readonly @expose_address _AtomicBoolRepr target,
    byte order);
    @extern("C") internal static extern void chic_rt_atomic_bool_store(* mut @expose_address _AtomicBoolRepr target,
    byte value, byte order);
    @extern("C") internal static extern byte chic_rt_atomic_bool_compare_exchange(* mut @expose_address _AtomicBoolRepr target,
    byte expected, byte desired, byte order);
    @extern("C") internal static extern usize chic_rt_atomic_usize_load(* const @readonly @expose_address _AtomicUsizeRepr target,
    byte order);
    @extern("C") internal static extern void chic_rt_atomic_usize_store(* mut @expose_address _AtomicUsizeRepr target,
    usize value, byte order);
    @extern("C") internal static extern usize chic_rt_atomic_usize_fetch_add(* mut @expose_address _AtomicUsizeRepr target,
    usize value, byte order);
    @extern("C") internal static extern usize chic_rt_atomic_usize_fetch_sub(* mut @expose_address _AtomicUsizeRepr target,
    usize value, byte order);
    @extern("C") internal static extern int chic_rt_atomic_i32_load(* const @readonly @expose_address _AtomicI32Repr target,
    byte order);
    @extern("C") internal static extern void chic_rt_atomic_i32_store(* mut @expose_address _AtomicI32Repr target,
    int value, byte order);
    @extern("C") internal static extern byte chic_rt_atomic_i32_compare_exchange(* mut @expose_address _AtomicI32Repr target,
    int expected, int desired, byte order);
    @extern("C") internal static extern int chic_rt_atomic_i32_fetch_add(* mut @expose_address _AtomicI32Repr target,
    int value, byte order);
    @extern("C") internal static extern int chic_rt_atomic_i32_fetch_sub(* mut @expose_address _AtomicI32Repr target,
    int value, byte order);
    @extern("C") internal static extern uint chic_rt_atomic_u32_load(* const @readonly @expose_address _AtomicU32Repr target,
    byte order);
    @extern("C") internal static extern void chic_rt_atomic_u32_store(* mut @expose_address _AtomicU32Repr target,
    uint value, byte order);
    @extern("C") internal static extern byte chic_rt_atomic_u32_compare_exchange(* mut @expose_address _AtomicU32Repr target,
    uint expected, uint desired, byte order);
    @extern("C") internal static extern uint chic_rt_atomic_u32_fetch_add(* mut @expose_address _AtomicU32Repr target,
    uint value, byte order);
    @extern("C") internal static extern uint chic_rt_atomic_u32_fetch_sub(* mut @expose_address _AtomicU32Repr target,
    uint value, byte order);
    @extern("C") internal static extern long chic_rt_atomic_i64_load(* const @readonly @expose_address _AtomicI64Repr target,
    byte order);
    @extern("C") internal static extern void chic_rt_atomic_i64_store(* mut @expose_address _AtomicI64Repr target,
    long value, byte order);
    @extern("C") internal static extern byte chic_rt_atomic_i64_compare_exchange(* mut @expose_address _AtomicI64Repr target,
    long expected, long desired, byte order);
    @extern("C") internal static extern long chic_rt_atomic_i64_fetch_add(* mut @expose_address _AtomicI64Repr target,
    long value, byte order);
    @extern("C") internal static extern long chic_rt_atomic_i64_fetch_sub(* mut @expose_address _AtomicI64Repr target,
    long value, byte order);
    @extern("C") internal static extern ulong chic_rt_atomic_u64_load(* const @readonly @expose_address _AtomicU64Repr target,
    byte order);
    @extern("C") internal static extern void chic_rt_atomic_u64_store(* mut @expose_address _AtomicU64Repr target,
    ulong value, byte order);
    @extern("C") internal static extern byte chic_rt_atomic_u64_compare_exchange(* mut @expose_address _AtomicU64Repr target,
    ulong expected, ulong desired, byte order);
    @extern("C") internal static extern ulong chic_rt_atomic_u64_fetch_add(* mut @expose_address _AtomicU64Repr target,
    ulong value, byte order);
    @extern("C") internal static extern ulong chic_rt_atomic_u64_fetch_sub(* mut @expose_address _AtomicU64Repr target,
    ulong value, byte order);
}
public struct Arc <T >
{
    private __StdSyncArcHandle _handle;
    public init(__StdSyncArcHandle handle) {
        _handle = handle;
    }
    public init(T value) {
        var handle = CoreIntrinsics.DefaultValue <__StdSyncArcHandle >();
        var slot = Std.Memory.MaybeUninit <T >.Init(value);
        unsafe {
            let payload = slot.AsValueConstPtr();
            var size = payload.Size;
            var align = payload.Alignment;
            if (size == 0usize)
            {
                size = (usize) __sizeof <nuint >();
            }
            if (align == 0usize)
            {
                align = (usize) __alignof <nuint >();
            }
            var status = RuntimeIntrinsics.chic_rt_arc_new(& handle, payload.Pointer, size, align, DropGlueOf <T >(),
            0);
            if (status != 0)
            {
                handle = CoreIntrinsics.DefaultValue <__StdSyncArcHandle >();
            }
        }
        _handle = handle;
    }
    private static isize DropGlueOf <T >() {
        return(isize) __drop_glue_of <T >();
    }
    public Arc <T >Clone() throws InvalidOperationException {
        EnsureHandleLive();
        var handle = CoreIntrinsics.DefaultValue <__StdSyncArcHandle >();
        unsafe {
            let status = RuntimeIntrinsics.chic_rt_arc_clone(& handle, & _handle);
            if (status != 0)
            {
                handle = CoreIntrinsics.DefaultValue <__StdSyncArcHandle >();
            }
        }
        return new Arc <T >(handle);
    }
    public ref T Value {
        get {
            EnsureHandleLive();
            var handle = _handle;
            return ArcRuntime.AsRef <T >(ref handle);
        }
    }
    public ref T Borrow() throws InvalidOperationException {
        EnsureHandleLive();
        var handle = _handle;
        return ArcRuntime.AsRef <T >(ref handle);
    }
    public ValueMutPtr AsValueMutPtr() throws InvalidOperationException {
        EnsureHandleLive();
        unsafe {
            let ptr = RuntimeIntrinsics.chic_rt_arc_get_mut(& _handle);
            if (__StdSyncPointerHelpers.IsNull (ptr))
            {
                throw new InvalidOperationException(StringRuntime.FromStr("Arc handle is null"));
            }
            var size = (usize) __sizeof <T >();
            var align = (usize) __alignof <T >();
            if (size == 0)
            {
                size = (usize)(__sizeof <nuint >() * 2);
            }
            if (align == 0)
            {
                align = (usize) __alignof <nuint >();
            }
            return ValuePointer.CreateMut(PointerIntrinsics.AsByteMut(ptr), size, align);
        }
    }
    public ValueConstPtr AsValueConstPtr() throws InvalidOperationException {
        EnsureHandleLive();
        unsafe {
            let ptr = RuntimeIntrinsics.chic_rt_arc_get(& _handle);
            if (__StdSyncPointerHelpers.IsNullConst (ptr))
            {
                throw new InvalidOperationException(StringRuntime.FromStr("Arc handle is null"));
            }
            var size = (usize) __sizeof <T >();
            var align = (usize) __alignof <T >();
            if (size == 0)
            {
                size = (usize)(__sizeof <nuint >() * 2);
            }
            if (align == 0)
            {
                align = (usize) __alignof <nuint >();
            }
            return ValuePointer.CreateConst(PointerIntrinsics.AsByteConst(ptr), size, align);
        }
    }
    public ValueMutPtr IntoRaw() throws InvalidOperationException {
        EnsureHandleLive();
        unsafe {
            return ValuePointer.CreateMut(PointerIntrinsics.AsByteMut(& _handle), (usize) __sizeof <__StdSyncArcHandle >(),
            (usize) __alignof <__StdSyncArcHandle >());
        }
    }
    public static Arc <T >FromRaw(ValueMutPtr raw) throws InvalidOperationException {
        let expectedSize = (usize) __sizeof <__StdSyncArcHandle >();
        let expectedAlign = (usize) __alignof <__StdSyncArcHandle >();
        if (ValuePointer.IsNullMut (raw))
        {
            var handle = CoreIntrinsics.DefaultValue <__StdSyncArcHandle >();
            return new Arc <T >(handle);
        }
        var handle = CoreIntrinsics.DefaultValue <__StdSyncArcHandle >();
        unsafe {
            let dest = ValuePointer.CreateMut(PointerIntrinsics.AsByteMut(& handle), expectedSize, expectedAlign);
            let source = ValuePointer.CreateConst(PointerIntrinsics.AsByteConstFromMut(raw.Pointer), expectedSize, expectedAlign);
            GlobalAllocator.Copy(dest, source, expectedSize);
        }
        return new Arc <T >(handle);
    }
    public void dispose(ref var this) {
        if (! __StdSyncPointerHelpers.IsNull (_handle.Pointer))
        {
            unsafe {
                RuntimeIntrinsics.chic_rt_arc_drop(& _handle);
            }
        }
        _handle = CoreIntrinsics.DefaultValue <__StdSyncArcHandle >();
    }
    public Weak <T >Downgrade() throws InvalidOperationException {
        EnsureHandleLive();
        var weak = CoreIntrinsics.DefaultValue <__StdSyncWeakHandle >();
        unsafe {
            let status = RuntimeIntrinsics.chic_rt_arc_downgrade(& weak, & _handle);
            if (status != 0)
            {
                weak = CoreIntrinsics.DefaultValue <__StdSyncWeakHandle >();
            }
        }
        return new Weak <T >(weak);
    }
    public T Value {
        get {
            var tmp = Std.Memory.MaybeUninit <T >.Uninit();
            unsafe {
                let source = AsValueConstPtr();
                GlobalAllocator.Copy(tmp.AsValueMutPtr(), source, source.Size);
            }
            tmp.MarkInitialized();
            return tmp.AssumeInit();
        }
    }
    private static void ValidateArcRaw(ValueMutPtr raw) throws InvalidOperationException {
        let expectedSize = (usize) __sizeof <__StdSyncArcHandle >();
        let expectedAlign = (usize) __alignof <__StdSyncArcHandle >();
        if (ValuePointer.IsNullMut (raw) || raw.Size != expectedSize || raw.Alignment != expectedAlign)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("invalid Arc handle layout"));
        }
    }
    private void EnsureHandleLive() throws InvalidOperationException {
        if (__StdSyncPointerHelpers.IsNull (_handle.Pointer))
        {
            throw new InvalidOperationException(StringRuntime.FromStr("Arc handle is null"));
        }
    }
}
public struct Weak <T >
{
    private __StdSyncWeakHandle _handle;
    public init() {
        _handle = CoreIntrinsics.DefaultValue <__StdSyncWeakHandle >();
    }
    internal init(__StdSyncWeakHandle handle) {
        _handle = handle;
    }
    public Weak <T >Clone() {
        var handle = CoreIntrinsics.DefaultValue <__StdSyncWeakHandle >();
        unsafe {
            let status = RuntimeIntrinsics.chic_rt_weak_clone(& handle, & _handle);
            if (status != 0)
            {
                handle = CoreIntrinsics.DefaultValue <__StdSyncWeakHandle >();
            }
        }
        return new Weak <T >(handle);
    }
    public Arc <T >?Upgrade() {
        var arc = CoreIntrinsics.DefaultValue <__StdSyncArcHandle >();
        unsafe {
            let status = RuntimeIntrinsics.chic_rt_weak_upgrade(& arc, & _handle);
            if (status != 0 || __StdSyncPointerHelpers.IsNull (arc.Pointer))
            {
                return null;
            }
        }
        return new Arc <T >(arc);
    }
    public void dispose(ref this) {
        if (! __StdSyncPointerHelpers.IsNull (_handle.Pointer))
        {
            unsafe {
                RuntimeIntrinsics.chic_rt_weak_drop(& _handle);
            }
        }
        _handle = CoreIntrinsics.DefaultValue <__StdSyncWeakHandle >();
    }
}
public sealed class Rc <T >
{
    private __StdSyncRcHandle _handle;
    public init(T value) {
        var handle = CoreIntrinsics.DefaultValue <__StdSyncRcHandle >();
        var slot = Std.Memory.MaybeUninit <T >.Init(value);
        unsafe {
            let payload = slot.AsValueConstPtr();
            var size = payload.Size;
            var align = payload.Alignment;
            if (size == 0usize)
            {
                size = (usize) __sizeof <nuint >();
            }
            if (align == 0usize)
            {
                align = (usize) __alignof <nuint >();
            }
            let status = RuntimeIntrinsics.chic_rt_rc_new(& handle, payload.Pointer, size, align, Std.Numeric.NumericUnchecked.ToUSize((isize) __drop_glue_of <T >()),
            0);
            if (status != 0)
            {
                handle = CoreIntrinsics.DefaultValue <__StdSyncRcHandle >();
            }
        }
        _handle = handle;
    }
    public Rc <T >Clone() throws InvalidOperationException {
        EnsureHandleLive();
        var handle = CoreIntrinsics.DefaultValue <__StdSyncRcHandle >();
        unsafe {
            let status = RuntimeIntrinsics.chic_rt_rc_clone(& handle, & _handle);
            if (status != 0)
            {
                handle = CoreIntrinsics.DefaultValue <__StdSyncRcHandle >();
            }
        }
        return new Rc <T >(handle);
    }
    private init(__StdSyncRcHandle handle) {
        _handle = handle;
    }
    public ValueConstPtr AsValueConstPtr() throws InvalidOperationException {
        EnsureHandleLive();
        unsafe {
            let ptr = RuntimeIntrinsics.chic_rt_rc_get(& _handle);
            if (__StdSyncPointerHelpers.IsNullConst (ptr))
            {
                throw new InvalidOperationException(StringRuntime.FromStr("Rc handle is null"));
            }
            var size = (usize) __sizeof <T >();
            var align = (usize) __alignof <T >();
            if (size == 0)
            {
                size = (usize)(__sizeof <nuint >() * 2);
            }
            if (align == 0)
            {
                align = (usize) __alignof <nuint >();
            }
            return ValuePointer.CreateConst(PointerIntrinsics.AsByteConst(ptr), size, align);
        }
    }
    public ValueMutPtr AsValueMutPtr() throws InvalidOperationException {
        EnsureHandleLive();
        unsafe {
            let ptr = RuntimeIntrinsics.chic_rt_rc_get_mut(& _handle);
            if (__StdSyncPointerHelpers.IsNull (ptr))
            {
                throw new InvalidOperationException(StringRuntime.FromStr("Rc handle is not uniquely owned"));
            }
            var size = (usize) __sizeof <T >();
            var align = (usize) __alignof <T >();
            if (size == 0)
            {
                size = (usize)(__sizeof <nuint >() * 2);
            }
            if (align == 0)
            {
                align = (usize) __alignof <nuint >();
            }
            return ValuePointer.CreateMut(PointerIntrinsics.AsByteMut(ptr), size, align);
        }
    }
    public ValueMutPtr IntoRaw() throws InvalidOperationException {
        EnsureHandleLive();
        unsafe {
            return ValuePointer.CreateMut(PointerIntrinsics.AsByteMut(& _handle), (usize) __sizeof <__StdSyncRcHandle >(),
            (usize) __alignof <__StdSyncRcHandle >());
        }
    }
    public static Rc <T >FromRaw(ValueMutPtr raw) throws InvalidOperationException {
        ValidateRcRaw(raw);
        var handle = CoreIntrinsics.DefaultValue <__StdSyncRcHandle >();
        unsafe {
            let dest = ValuePointer.CreateMut(PointerIntrinsics.AsByteMut(& handle), (usize) __sizeof <__StdSyncRcHandle >(),
            (usize) __alignof <__StdSyncRcHandle >());
            let source = ValuePointer.CreateConst(PointerIntrinsics.AsByteConstFromMut(raw.Pointer), raw.Size, raw.Alignment);
            GlobalAllocator.Copy(dest, source, dest.Size);
        }
        return new Rc <T >(handle);
    }
    public void dispose(ref this) {
        if (! __StdSyncPointerHelpers.IsNull (_handle.Pointer))
        {
            unsafe {
                RuntimeIntrinsics.chic_rt_rc_drop(& _handle);
            }
        }
        _handle = CoreIntrinsics.DefaultValue <__StdSyncRcHandle >();
    }
    public WeakRc <T >Downgrade() throws InvalidOperationException {
        EnsureHandleLive();
        var weak = CoreIntrinsics.DefaultValue <__StdSyncWeakRcHandle >();
        unsafe {
            let status = RuntimeIntrinsics.chic_rt_rc_downgrade(& weak, & _handle);
            if (status != 0)
            {
                weak = CoreIntrinsics.DefaultValue <__StdSyncWeakRcHandle >();
            }
        }
        return new WeakRc <T >(weak);
    }
    public T Value {
        get {
            EnsureHandleLive();
            var tmp = Std.Memory.MaybeUninit <T >.Uninit();
            unsafe {
                let source = AsValueConstPtr();
                GlobalAllocator.Copy(tmp.AsValueMutPtr(), source, source.Size);
            }
            tmp.MarkInitialized();
            return tmp.AssumeInit();
        }
    }
    private static void ValidateRcRaw(ValueMutPtr raw) throws InvalidOperationException {
        let expectedSize = (usize) __sizeof <__StdSyncRcHandle >();
        let expectedAlign = (usize) __alignof <__StdSyncRcHandle >();
        if (ValuePointer.IsNullMut (raw) || raw.Size != expectedSize || raw.Alignment != expectedAlign)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("invalid Rc handle layout"));
        }
    }
    private void EnsureHandleLive() throws InvalidOperationException {
        if (__StdSyncPointerHelpers.IsNull (_handle.Pointer))
        {
            throw new InvalidOperationException(StringRuntime.FromStr("Rc handle is null"));
        }
    }
}
public sealed class WeakRc <T >
{
    private __StdSyncWeakRcHandle _handle;
    internal init(__StdSyncWeakRcHandle handle) {
        _handle = handle;
    }
    public init() {
        _handle = CoreIntrinsics.DefaultValue <__StdSyncWeakRcHandle >();
    }
    public WeakRc <T >Clone() {
        var handle = CoreIntrinsics.DefaultValue <__StdSyncWeakRcHandle >();
        unsafe {
            let status = RuntimeIntrinsics.chic_rt_weak_rc_clone(& handle, & _handle);
            if (status != 0)
            {
                handle = CoreIntrinsics.DefaultValue <__StdSyncWeakRcHandle >();
            }
        }
        return new WeakRc <T >(handle);
    }
    public Rc <T >?Upgrade() {
        var rc = CoreIntrinsics.DefaultValue <__StdSyncRcHandle >();
        unsafe {
            let status = RuntimeIntrinsics.chic_rt_weak_rc_upgrade(& rc, & _handle);
            if (status != 0 || __StdSyncPointerHelpers.IsNull (rc.Pointer))
            {
                return null;
            }
        }
        return new Rc <T >(rc);
    }
    public void dispose(ref this) {
        if (! __StdSyncPointerHelpers.IsNull (_handle.Pointer))
        {
            unsafe {
                RuntimeIntrinsics.chic_rt_weak_rc_drop(& _handle);
            }
        }
        _handle = CoreIntrinsics.DefaultValue <__StdSyncWeakRcHandle >();
    }
}
