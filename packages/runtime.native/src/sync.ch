namespace Std.Runtime.Native;
// Chic-native synchronization and atomic intrinsics. These are minimal,
// single-threaded-friendly implementations that satisfy the frozen
// `chic_rt_*` ABI when the native runtime archive is linked.
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
@repr(c) internal struct MutexState
{
    // Align the embedded pthread object to at least pointer width.
    public usize Align;
    public InlineBytes256 MutexBytes;
    public byte Initialized;
    public byte Held;
    public byte Pad0;
    public byte Pad1;
    public byte Pad2;
    public byte Pad3;
    public byte Pad4;
    public byte Pad5;
}
@repr(c) public struct InlineBytes256
{
    public InlineBytes64 b0;
    public InlineBytes64 b1;
    public InlineBytes64 b2;
    public InlineBytes64 b3;
}
@repr(c) internal struct RwLockState
{
    // Align the embedded pthread object to at least pointer width.
    public usize Align;
    public InlineBytes256 RwlockBytes;
    public byte Initialized;
    public byte Pad0;
    public byte Pad1;
    public byte Pad2;
    public byte Pad3;
    public byte Pad4;
    public byte Pad5;
    public byte Pad6;
}
@repr(c) internal struct CondvarState
{
    public usize Waiters;
}
@repr(c) internal struct OnceState
{
    public byte State;
}
private static class PThread
{
    @extern("C") public unsafe static extern int pthread_mutex_init(* mut @expose_address byte mutex, * const @readonly @expose_address byte attr);
    @extern("C") public unsafe static extern int pthread_mutex_destroy(* mut @expose_address byte mutex);
    @extern("C") public unsafe static extern int pthread_mutex_lock(* mut @expose_address byte mutex);
    @extern("C") public unsafe static extern int pthread_mutex_trylock(* mut @expose_address byte mutex);
    @extern("C") public unsafe static extern int pthread_mutex_unlock(* mut @expose_address byte mutex);
    @extern("C") public unsafe static extern int pthread_rwlock_init(* mut @expose_address byte rwlock, * const @readonly @expose_address byte attr);
    @extern("C") public unsafe static extern int pthread_rwlock_destroy(* mut @expose_address byte rwlock);
    @extern("C") public unsafe static extern int pthread_rwlock_rdlock(* mut @expose_address byte rwlock);
    @extern("C") public unsafe static extern int pthread_rwlock_tryrdlock(* mut @expose_address byte rwlock);
    @extern("C") public unsafe static extern int pthread_rwlock_wrlock(* mut @expose_address byte rwlock);
    @extern("C") public unsafe static extern int pthread_rwlock_trywrlock(* mut @expose_address byte rwlock);
    @extern("C") public unsafe static extern int pthread_rwlock_unlock(* mut @expose_address byte rwlock);
}
private unsafe static usize PtrToHandle(* mut @expose_address byte ptr) {
    return(usize) NativePtr.ToIsize(ptr);
}
private unsafe static * mut @expose_address MutexState MutexFromHandle(usize handle) {
    return NativePtr.FromIsize((isize) handle);
}
private unsafe static * mut @expose_address RwLockState RwLockFromHandle(usize handle) {
    return NativePtr.FromIsize((isize) handle);
}
private unsafe static * mut @expose_address CondvarState CondvarFromHandle(usize handle) {
    return NativePtr.FromIsize((isize) handle);
}
private unsafe static * mut @expose_address byte OnceFromHandle(usize handle) {
    return NativePtr.FromIsize((isize) handle);
}
private unsafe static * mut @expose_address byte RwLockNativePtr(* mut @expose_address RwLockState state) {
    return(* mut @expose_address byte) & mut(* state).RwlockBytes;
}
private unsafe static * mut @expose_address byte MutexNativePtr(* mut @expose_address MutexState state) {
    return(* mut @expose_address byte) & mut(* state).MutexBytes;
}
// Mutex / Lock ----------------------------------------------------------------
@extern("C") @export("chic_rt_mutex_create") public unsafe static usize chic_rt_mutex_create() {
    let size = sizeof(MutexState);
    let align = __alignof <MutexState >();
    var state = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = size, Alignment = align
    }
    ;
    if (NativeAlloc.AllocZeroed (size, align, out state) != NativeAllocationError.Success) {
        return 0;
    }
    let ptr = (* mut @expose_address MutexState) state.Pointer;
    if (PThread.pthread_mutex_init (MutexNativePtr (ptr), NativePtr.NullConst ()) != 0)
    {
        NativeAlloc.Free(state);
        return 0;
    }
    (* ptr).Initialized = 1u8;
    return PtrToHandle(state.Pointer);
}
@extern("C") @export("chic_rt_mutex_destroy") public unsafe static void chic_rt_mutex_destroy(usize handle) {
    if (handle == 0usize)
    {
        return;
    }
    let ptr = MutexFromHandle(handle);
    if (NativePtr.IsNull (ptr))
    {
        return;
    }
    if ( (* ptr).Initialized != 0u8)
    {
        let _ = PThread.pthread_mutex_destroy(MutexNativePtr(ptr));
    }
    NativeAlloc.Free(new ValueMutPtr {
        Pointer = (* mut @expose_address byte) ptr, Size = sizeof(MutexState), Alignment = __alignof <MutexState >()
    }
    );
}
@extern("C") @export("chic_rt_mutex_lock") public unsafe static void chic_rt_mutex_lock(usize handle) {
    var ptr = MutexFromHandle(handle);
    if (NativePtr.IsNull (ptr))
    {
        return;
    }
    let _ = PThread.pthread_mutex_lock(MutexNativePtr(ptr));
    (* ptr).Held = 1u8;
}
@extern("C") @export("chic_rt_mutex_try_lock") public unsafe static bool chic_rt_mutex_try_lock(usize handle) {
    var ptr = MutexFromHandle(handle);
    if (NativePtr.IsNull (ptr))
    {
        return false;
    }
    if (PThread.pthread_mutex_trylock (MutexNativePtr (ptr)) != 0)
    {
        return false;
    }
    (* ptr).Held = 1u8;
    return true;
}
@extern("C") @export("chic_rt_mutex_unlock") public unsafe static void chic_rt_mutex_unlock(usize handle) {
    var ptr = MutexFromHandle(handle);
    if (NativePtr.IsNull (ptr))
    {
        return;
    }
    let _ = PThread.pthread_mutex_unlock(MutexNativePtr(ptr));
    (* ptr).Held = 0u8;
}
// Lock aliases mirror mutex semantics
@extern("C") @export("chic_rt_lock_create") public unsafe static usize chic_rt_lock_create() => chic_rt_mutex_create();
@extern("C") @export("chic_rt_lock_destroy") public unsafe static void chic_rt_lock_destroy(usize handle) => chic_rt_mutex_destroy(handle);
@extern("C") @export("chic_rt_lock_enter") public unsafe static void chic_rt_lock_enter(usize handle) => chic_rt_mutex_lock(handle);
@extern("C") @export("chic_rt_lock_try_enter") public unsafe static bool chic_rt_lock_try_enter(usize handle) => chic_rt_mutex_try_lock(handle);
@extern("C") @export("chic_rt_lock_exit") public unsafe static void chic_rt_lock_exit(usize handle) => chic_rt_mutex_unlock(handle);
@extern("C") @export("chic_rt_lock_is_held") public unsafe static bool chic_rt_lock_is_held(usize handle) {
    var ptr = MutexFromHandle(handle);
    if (NativePtr.IsNull (ptr))
    {
        return false;
    }
    return(* ptr).Held != 0u8;
}
@extern("C") @export("chic_rt_lock_is_held_by_current_thread") public unsafe static bool chic_rt_lock_is_held_by_current_thread(usize handle) {
    // No thread tracking in native runtime; mirror lock_is_held semantics.
    return chic_rt_lock_is_held(handle);
}
// RWLock ----------------------------------------------------------------------
@extern("C") @export("chic_rt_rwlock_create") public unsafe static usize chic_rt_rwlock_create() {
    let size = sizeof(RwLockState);
    let align = __alignof <RwLockState >();
    var state = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = size, Alignment = align
    }
    ;
    if (NativeAlloc.AllocZeroed (size, align, out state) != NativeAllocationError.Success) {
        return 0;
    }
    let ptr = (* mut @expose_address RwLockState) state.Pointer;
    if (PThread.pthread_rwlock_init (RwLockNativePtr (ptr), NativePtr.NullConst ()) != 0)
    {
        NativeAlloc.Free(state);
        return 0;
    }
    (* ptr).Initialized = 1u8;
    return PtrToHandle(state.Pointer);
}
@extern("C") @export("chic_rt_rwlock_destroy") public unsafe static void chic_rt_rwlock_destroy(usize handle) {
    if (handle == 0usize)
    {
        return;
    }
    let ptr = RwLockFromHandle(handle);
    if (!NativePtr.IsNull (ptr))
    {
        if ( (* ptr).Initialized != 0u8)
        {
            let _ = PThread.pthread_rwlock_destroy(RwLockNativePtr(ptr));
        }
        NativeAlloc.Free(new ValueMutPtr {
            Pointer = (* mut @expose_address byte) ptr, Size = sizeof(RwLockState), Alignment = __alignof <RwLockState >()
        }
        );
    }
}
@extern("C") @export("chic_rt_rwlock_read_lock") public unsafe static void chic_rt_rwlock_read_lock(usize handle) {
    var ptr = RwLockFromHandle(handle);
    if (NativePtr.IsNull (ptr))
    {
        return;
    }
    let _ = PThread.pthread_rwlock_rdlock(RwLockNativePtr(ptr));
}
@extern("C") @export("chic_rt_rwlock_try_read_lock") public unsafe static bool chic_rt_rwlock_try_read_lock(usize handle) {
    var ptr = RwLockFromHandle(handle);
    if (NativePtr.IsNull (ptr))
    {
        return false;
    }
    return PThread.pthread_rwlock_tryrdlock(RwLockNativePtr(ptr)) == 0;
}
@extern("C") @export("chic_rt_rwlock_read_unlock") public unsafe static void chic_rt_rwlock_read_unlock(usize handle) {
    var ptr = RwLockFromHandle(handle);
    if (NativePtr.IsNull (ptr))
    {
        return;
    }
    let _ = PThread.pthread_rwlock_unlock(RwLockNativePtr(ptr));
}
@extern("C") @export("chic_rt_rwlock_write_lock") public unsafe static void chic_rt_rwlock_write_lock(usize handle) {
    var ptr = RwLockFromHandle(handle);
    if (NativePtr.IsNull (ptr))
    {
        return;
    }
    let _ = PThread.pthread_rwlock_wrlock(RwLockNativePtr(ptr));
}
@extern("C") @export("chic_rt_rwlock_try_write_lock") public unsafe static bool chic_rt_rwlock_try_write_lock(usize handle) {
    var ptr = RwLockFromHandle(handle);
    if (NativePtr.IsNull (ptr))
    {
        return false;
    }
    return PThread.pthread_rwlock_trywrlock(RwLockNativePtr(ptr)) == 0;
}
@extern("C") @export("chic_rt_rwlock_write_unlock") public unsafe static void chic_rt_rwlock_write_unlock(usize handle) {
    var ptr = RwLockFromHandle(handle);
    if (NativePtr.IsNull (ptr))
    {
        return;
    }
    let _ = PThread.pthread_rwlock_unlock(RwLockNativePtr(ptr));
}
// Condvar ---------------------------------------------------------------------
@extern("C") @export("chic_rt_condvar_create") public unsafe static usize chic_rt_condvar_create() {
    let size = sizeof(CondvarState);
    var state = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = size, Alignment = 1
    }
    ;
    if (NativeAlloc.AllocZeroed (size, 1, out state) != NativeAllocationError.Success) {
        return 0;
    }
    return PtrToHandle(state.Pointer);
}
@extern("C") @export("chic_rt_condvar_destroy") public unsafe static void chic_rt_condvar_destroy(usize handle) {
    if (handle == 0usize)
    {
        return;
    }
    let ptr = CondvarFromHandle(handle);
    if (!NativePtr.IsNull (ptr))
    {
        NativeAlloc.Free(new ValueMutPtr {
            Pointer = ptr, Size = sizeof(CondvarState), Alignment = 1
        }
        );
    }
}
@extern("C") @export("chic_rt_condvar_notify_one") public unsafe static void chic_rt_condvar_notify_one(usize _handle) {
}
@extern("C") @export("chic_rt_condvar_notify_all") public unsafe static void chic_rt_condvar_notify_all(usize _handle) {
}
@extern("C") @export("chic_rt_condvar_wait") public unsafe static void chic_rt_condvar_wait(usize _condvar_handle, usize _mutex_handle) {
}
// Once -----------------------------------------------------------------------
@extern("C") @export("chic_rt_once_create") public unsafe static usize chic_rt_once_create() {
    let size = sizeof(OnceState);
    let align = __alignof <OnceState >();
    var state = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = size, Alignment = align
    }
    ;
    if (NativeAlloc.AllocZeroed (size, align, out state) != NativeAllocationError.Success) {
        return 0;
    }
    return PtrToHandle(state.Pointer);
}
@extern("C") @export("chic_rt_once_destroy") public unsafe static void chic_rt_once_destroy(usize handle) {
    if (handle == 0usize)
    {
        return;
    }
    var ptr = OnceFromHandle(handle);
    if (!NativePtr.IsNull (ptr))
    {
        let size = sizeof(OnceState);
        let align = __alignof <OnceState >();
        NativeAlloc.Free(new ValueMutPtr {
            Pointer = ptr, Size = size, Alignment = align
        }
        );
    }
}
@extern("C") @export("chic_rt_once_try_begin") public unsafe static bool chic_rt_once_try_begin(usize handle) {
    var ptr = OnceFromHandle(handle);
    if (NativePtr.IsNull (ptr))
    {
        return false;
    }
    if (* ptr != 0)
    {
        return false;
    }
    * ptr = 1;
    return true;
}
@extern("C") @export("chic_rt_once_complete") public unsafe static void chic_rt_once_complete(usize handle) {
    var ptr = OnceFromHandle(handle);
    if (NativePtr.IsNull (ptr))
    {
        return;
    }
    * ptr = 2u8;
}
@extern("C") @export("chic_rt_once_wait") public unsafe static void chic_rt_once_wait(usize handle) {
    var ptr = OnceFromHandle(handle);
    if (NativePtr.IsNull (ptr))
    {
        return;
    }
    while (* ptr != 2)
    {
    }
}
@extern("C") @export("chic_rt_once_is_completed") public unsafe static bool chic_rt_once_is_completed(usize handle) {
    var ptr = OnceFromHandle(handle);
    if (NativePtr.IsNull (ptr))
    {
        return false;
    }
    return * ptr == 2;
}
// Atomics --------------------------------------------------------------------
@extern("C") @export("chic_rt_atomic_bool_load") public unsafe static byte chic_rt_atomic_bool_load(* const @readonly @expose_address byte target,
byte _order) {
    if (target == null)
    {
        return 0;
    }
    return * target;
}
@extern("C") @export("chic_rt_atomic_bool_store") public unsafe static void chic_rt_atomic_bool_store(* mut @expose_address byte target,
byte value, byte _order) {
    if (target == null)
    {
        return;
    }
    * target = value;
}
@extern("C") @export("chic_rt_atomic_bool_compare_exchange") public unsafe static byte chic_rt_atomic_bool_compare_exchange(* mut @expose_address byte target,
byte expected, byte desired, byte _order) {
    if (target == null)
    {
        return 0;
    }
    if (* target == expected)
    {
        * target = desired;
        return 1;
    }
    return 0;
}
@extern("C") @export("chic_rt_atomic_usize_load") public unsafe static usize chic_rt_atomic_usize_load(* const @readonly @expose_address usize target,
byte _order) {
    return target == null ?0usize : * target;
}
@extern("C") @export("chic_rt_atomic_usize_store") public unsafe static void chic_rt_atomic_usize_store(* mut @expose_address usize target,
usize value, byte _order) {
    if (target != null)
    {
        * target = value;
    }
}
@extern("C") @export("chic_rt_atomic_usize_fetch_add") public unsafe static usize chic_rt_atomic_usize_fetch_add(* mut @expose_address usize target,
usize value, byte _order) {
    if (target == null)
    {
        return 0usize;
    }
    let prior = * target;
    * target = prior + value;
    return prior;
}
@extern("C") @export("chic_rt_atomic_usize_fetch_sub") public unsafe static usize chic_rt_atomic_usize_fetch_sub(* mut @expose_address usize target,
usize value, byte _order) {
    if (target == null)
    {
        return 0usize;
    }
    let prior = * target;
    * target = prior - value;
    return prior;
}
@extern("C") @export("chic_rt_atomic_i32_load") public unsafe static int chic_rt_atomic_i32_load(* const @readonly @expose_address int target,
byte _order) {
    return target == null ?0 : * target;
}
@extern("C") @export("chic_rt_atomic_i32_store") public unsafe static void chic_rt_atomic_i32_store(* mut @expose_address int target,
int value, byte _order) {
    if (target != null)
    {
        * target = value;
    }
}
@extern("C") @export("chic_rt_atomic_i32_fetch_add") public unsafe static int chic_rt_atomic_i32_fetch_add(* mut @expose_address int target,
int value, byte _order) {
    if (target == null)
    {
        return 0;
    }
    let prior = * target;
    * target = prior + value;
    return prior;
}
@extern("C") @export("chic_rt_atomic_i32_fetch_sub") public unsafe static int chic_rt_atomic_i32_fetch_sub(* mut @expose_address int target,
int value, byte _order) {
    if (target == null)
    {
        return 0;
    }
    let prior = * target;
    * target = prior - value;
    return prior;
}
@extern("C") @export("chic_rt_atomic_i32_compare_exchange") public unsafe static byte chic_rt_atomic_i32_compare_exchange(* mut @expose_address int target,
int expected, int desired, byte _order) {
    if (target == null)
    {
        return 0;
    }
    if (* target == expected)
    {
        * target = desired;
        return 1;
    }
    return 0;
}
@extern("C") @export("chic_rt_atomic_u32_load") public unsafe static uint chic_rt_atomic_u32_load(* const @readonly @expose_address uint target,
byte _order) {
    return target == null ?0u : * target;
}
@extern("C") @export("chic_rt_atomic_u32_store") public unsafe static void chic_rt_atomic_u32_store(* mut @expose_address uint target,
uint value, byte _order) {
    if (target != null)
    {
        * target = value;
    }
}
@extern("C") @export("chic_rt_atomic_u32_fetch_add") public unsafe static uint chic_rt_atomic_u32_fetch_add(* mut @expose_address uint target,
uint value, byte _order) {
    if (target == null)
    {
        return 0u;
    }
    let prior = * target;
    * target = prior + value;
    return prior;
}
@extern("C") @export("chic_rt_atomic_u32_fetch_sub") public unsafe static uint chic_rt_atomic_u32_fetch_sub(* mut @expose_address uint target,
uint value, byte _order) {
    if (target == null)
    {
        return 0u;
    }
    let prior = * target;
    * target = prior - value;
    return prior;
}
@extern("C") @export("chic_rt_atomic_u32_compare_exchange") public unsafe static byte chic_rt_atomic_u32_compare_exchange(* mut @expose_address uint target,
uint expected, uint desired, byte _order) {
    if (target == null)
    {
        return 0;
    }
    if (* target == expected)
    {
        * target = desired;
        return 1;
    }
    return 0;
}
@extern("C") @export("chic_rt_atomic_i64_load") public unsafe static long chic_rt_atomic_i64_load(* const @readonly @expose_address long target,
byte _order) {
    return target == null ?0L : * target;
}
@extern("C") @export("chic_rt_atomic_i64_store") public unsafe static void chic_rt_atomic_i64_store(* mut @expose_address long target,
long value, byte _order) {
    if (target != null)
    {
        * target = value;
    }
}
@extern("C") @export("chic_rt_atomic_i64_fetch_add") public unsafe static long chic_rt_atomic_i64_fetch_add(* mut @expose_address long target,
long value, byte _order) {
    if (target == null)
    {
        return 0L;
    }
    let prior = * target;
    * target = prior + value;
    return prior;
}
@extern("C") @export("chic_rt_atomic_i64_fetch_sub") public unsafe static long chic_rt_atomic_i64_fetch_sub(* mut @expose_address long target,
long value, byte _order) {
    if (target == null)
    {
        return 0L;
    }
    let prior = * target;
    * target = prior - value;
    return prior;
}
@extern("C") @export("chic_rt_atomic_i64_compare_exchange") public unsafe static byte chic_rt_atomic_i64_compare_exchange(* mut @expose_address long target,
long expected, long desired, byte _order) {
    if (target == null)
    {
        return 0;
    }
    if (* target == expected)
    {
        * target = desired;
        return 1;
    }
    return 0;
}
@extern("C") @export("chic_rt_atomic_u64_load") public unsafe static ulong chic_rt_atomic_u64_load(* const @readonly @expose_address ulong target,
byte _order) {
    return target == null ?0ul : * target;
}
@extern("C") @export("chic_rt_atomic_u64_store") public unsafe static void chic_rt_atomic_u64_store(* mut @expose_address ulong target,
ulong value, byte _order) {
    if (target != null)
    {
        * target = value;
    }
}
@extern("C") @export("chic_rt_atomic_u64_fetch_add") public unsafe static ulong chic_rt_atomic_u64_fetch_add(* mut @expose_address ulong target,
ulong value, byte _order) {
    if (target == null)
    {
        return 0ul;
    }
    let prior = * target;
    * target = prior + value;
    return prior;
}
@extern("C") @export("chic_rt_atomic_u64_fetch_sub") public unsafe static ulong chic_rt_atomic_u64_fetch_sub(* mut @expose_address ulong target,
ulong value, byte _order) {
    if (target == null)
    {
        return 0ul;
    }
    let prior = * target;
    * target = prior - value;
    return prior;
}
@extern("C") @export("chic_rt_atomic_u64_compare_exchange") public unsafe static byte chic_rt_atomic_u64_compare_exchange(* mut @expose_address ulong target,
ulong expected, ulong desired, byte _order) {
    if (target == null)
    {
        return 0;
    }
    if (* target == expected)
    {
        * target = desired;
        return 1;
    }
    return 0;
}
