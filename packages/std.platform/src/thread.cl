namespace Std.Platform.Thread;
import Std;
import Std.Numeric;
import Std.Runtime;
import Std.Runtime.Collections;
import Std.Sync;
import Std.Core;
import Std.Memory;
import Std.Span;
import Std.Strings;
public interface ThreadStart
{
    void Run();
}
public sealed class ThreadFunctionStartAdapter : ThreadStart
{
    private ThreadStartCallback _fn;
    public init(ThreadStartCallback callback)
    {
        _fn = callback;
    }
    public void Run() {
        _fn();
    }
}
public sealed class ThreadFunctionRunner : ThreadStart
{
    private ThreadFunctionStartAdapter _inner;
    public init(ThreadStartCallback entry)
    {
        _inner = new ThreadFunctionStartAdapter(entry);
    }
    public void Run() {
        _inner.Run();
    }
}
public static class ThreadStartFactory
{
    public static Arc <T >From <T >(T runner) where T : ThreadStart {
        if (runner == null)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("thread start payload must not be null"));
        }
        return new Arc <T >(runner);
    }
    public static Arc <ThreadFunctionStartAdapter >Function(ThreadStartCallback entry)
    {
        if (entry == null)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("thread entry function must not be null"));
        }
        return new Arc <ThreadFunctionStartAdapter >(new ThreadFunctionStartAdapter(entry));
    }
}
public enum ThreadStatus
{
    Success = 0, NotSupported = 1, Invalid = 2, SpawnFailed = 3,
}
public static class ThreadStatusExtensions
{
    public static string ToString(this ThreadStatus status) {
        switch (status)
        {
        case ThreadStatus.Success:
            return StringRuntime.FromStr("Success");
        case ThreadStatus.NotSupported:
            return StringRuntime.FromStr("NotSupported");
        case ThreadStatus.Invalid:
            return StringRuntime.FromStr("Invalid");
        case ThreadStatus.SpawnFailed:
            return StringRuntime.FromStr("SpawnFailed");
        default :
            return StringRuntime.FromStr("Unknown");
        }
        }
        }
        @repr(c) internal struct ThreadStartDescriptor
        {
            public ValueMutPtr Context;
            public ValueMutPtr Name;
            public bool HasName;
            public bool UseThreadIdName;
        }
        @repr(c) public struct ThreadHandle
        {
            internal * mut @expose_address byte Raw;
            internal bool IsValid {
                get {
                    let handle = ValuePointer.CreateMut(PointerIntrinsics.AsByteMut(Raw),
                    0usize, 0usize);
                    return ! ValuePointer.IsNullMut(handle);
                }
            }
            internal void Clear() {
                let handle = ValuePointer.NullMut(0usize, 0usize);
                Raw = handle.Pointer;
            }
            internal static ThreadHandle Null() {
                var handle = CoreIntrinsics.DefaultValue <ThreadHandle >();
                let nullHandle = ValuePointer.NullMut(0usize, 0usize);
                handle.Raw = nullHandle.Pointer;
                return handle;
            }
        }
        internal static class RuntimeExports
        {
            @extern("C") public static extern ThreadStatus chic_rt_thread_spawn(ref ThreadStartDescriptor start, ref ThreadHandle handle);
            @extern("C") public static extern ThreadStatus chic_rt_thread_join(ref ThreadHandle handle);
            @extern("C") public static extern ThreadStatus chic_rt_thread_detach(ref ThreadHandle handle);
            @extern("C") public static extern void chic_rt_thread_sleep_ms(ulong milliseconds);
            @extern("C") public static extern void chic_rt_thread_yield();
            @extern("C") public static extern void chic_rt_thread_spin_wait(uint iterations);
        }
        internal static class RuntimeCallbacks
        {
            internal static bool ContextLayoutIsValid(ValueMutPtr context) {
                let expectedSize = (usize) __sizeof <Std.Sync.__StdSyncArcHandle >();
                let expectedAlign = (usize) __alignof <Std.Sync.__StdSyncArcHandle >();
                return ! Std.Runtime.Collections.ValuePointer.IsNullMut(context) && context.Size == expectedSize && context.Alignment == expectedAlign;
            }
            public static void Invoke(ValueMutPtr context) {
                if (! ContextLayoutIsValid (context))
                {
                    return;
                }
                var handle = Std.Sync.Arc <ThreadStart >.FromRaw(context);
                try {
                    var runner = handle.Value;
                    runner.Run();
                }
                finally {
                    handle.dispose();
                    if (! Std.Runtime.Collections.ValuePointer.IsNullMut (context))
                    {
                        Std.Memory.GlobalAllocator.Free(context);
                    }
                }
            }
            public static void Drop(ValueMutPtr context) {
                if (! ContextLayoutIsValid (context))
                {
                    return;
                }
                var handle = Std.Sync.Arc <ThreadStart >.FromRaw(context);
                handle.dispose();
                if (! Std.Runtime.Collections.ValuePointer.IsNullMut (context))
                {
                    Std.Memory.GlobalAllocator.Free(context);
                }
            }
        }
        public struct ThreadBuilder
        {
            private string _name;
            internal const int MaxThreadNameBytes = 15;
            public init() {
                _name = "";
            }
            public ThreadBuilder WithName(string name) {
                if (name == null)
                {
                    _name = "";
                }
                else
                {
                    _name = name;
                }
                return this;
            }
            public Thread Spawn <T >(Arc <T >start) where T : ThreadStart {
                return Thread.SpawnNamed(start, _name, MaxThreadNameBytes, ShouldUseThreadIdName());
            }
            private static bool ShouldUseThreadIdName() {
                let os = Std.Platform.EnvironmentInfo.OsDescription();
                if (os == null)
                {
                    return false;
                }
                // Avoid full locale-aware lowercasing; we only need a simple contains check.
                if (os.IndexOf ("linux") >= 0)
                {
                    return true;
                }
                if (os.IndexOf ("Linux") >= 0)
                {
                    return true;
                }
                return os.IndexOf("LINUX") >= 0;
            }
        }
        @repr(c) public struct Thread
        {
            private ThreadHandle _handle;
            private bool _hasHandle;
            private bool _completed;
            private string _name;
            public string Name {
                get {
                    return _name;
                }
            }
            public static Thread Spawn <T >(Arc <T >start) where T : ThreadStart {
                return SpawnNamed(start, "", ThreadBuilder.MaxThreadNameBytes, false);
            }
            internal static Thread SpawnNamed <T >(Arc <T >start, string name, int maxNameBytes, bool useThreadIdName) where T : ThreadStart {
                var thread = CoreIntrinsics.DefaultValue <Thread >();
                thread._handle = ThreadHandle.Null();
                thread._hasHandle = true;
                thread._completed = false;
                thread._name = name == null ?"" : name;
                unsafe {
                    let sourceHandle = start.IntoRaw();
                    var sourceHandleValue = CoreIntrinsics.DefaultValue <Std.Sync.__StdSyncArcHandle >();
                    let tmpDest = Std.Runtime.Collections.ValuePointer.CreateMut(Std.Numeric.PointerIntrinsics.AsByteMut(& sourceHandleValue),
                    (usize) __sizeof <Std.Sync.__StdSyncArcHandle >(), (usize) __alignof <Std.Sync.__StdSyncArcHandle >());
                    let tmpSrc = Std.Runtime.Collections.ValuePointer.CreateConst(Std.Numeric.PointerIntrinsics.AsByteConstFromMut(sourceHandle.Pointer),
                    (usize) __sizeof <Std.Sync.__StdSyncArcHandle >(), (usize) __alignof <Std.Sync.__StdSyncArcHandle >());
                    Std.Memory.GlobalAllocator.Copy(tmpDest, tmpSrc, tmpDest.Size);
                    var cloned = CoreIntrinsics.DefaultValue <Std.Sync.__StdSyncArcHandle >();
                    let cloneStatus = Std.Sync.RuntimeIntrinsics.chic_rt_arc_clone(& cloned, & sourceHandleValue);
                    if (cloneStatus != 0)
                    {
                        ThrowSpawn(ThreadStatus.SpawnFailed);
                    }
                    var dest = Std.Runtime.Collections.ValuePointer.NullMut((usize) __sizeof <Std.Sync.__StdSyncArcHandle >(),
                    (usize) __alignof <Std.Sync.__StdSyncArcHandle >());
                    let allocStatus = Std.Memory.GlobalAllocator.AllocZeroed((usize) __sizeof <Std.Sync.__StdSyncArcHandle >(),
                    (usize) __alignof <Std.Sync.__StdSyncArcHandle >(), out dest);
                    if (allocStatus != AllocationError.Success)
                    {
                        Std.Sync.RuntimeIntrinsics.chic_rt_arc_drop(& cloned);
                        ThrowSpawn(ThreadStatus.SpawnFailed);
                    }
                    var startDescriptor = CoreIntrinsics.DefaultValue <ThreadStartDescriptor >();
                    startDescriptor.Context = dest;
                    let sourceClone = Std.Runtime.Collections.ValuePointer.CreateConst(Std.Numeric.PointerIntrinsics.AsByteConst(& cloned),
                    (usize) __sizeof <Std.Sync.__StdSyncArcHandle >(), (usize) __alignof <Std.Sync.__StdSyncArcHandle >());
                    Std.Memory.GlobalAllocator.Copy(dest, sourceClone, dest.Size);
                    var nameHandle = Std.Runtime.Collections.ValuePointer.NullMut(1usize, 1usize);
                    var hasName = false;
                    thread._name = NormalizeName(name, maxNameBytes, out nameHandle, out hasName);
                    startDescriptor.Name = nameHandle;
                    startDescriptor.HasName = hasName;
                    startDescriptor.UseThreadIdName = useThreadIdName && hasName;
                    var status = RuntimeExports.chic_rt_thread_spawn(ref startDescriptor, ref thread._handle);
                    if (status != ThreadStatus.Success)
                    {
                        thread._hasHandle = false;
                        thread._handle.Clear();
                        if (hasName && ! Std.Runtime.Collections.ValuePointer.IsNullMut (nameHandle))
                        {
                            Std.Memory.GlobalAllocator.Free(nameHandle);
                        }
                        Std.Sync.RuntimeIntrinsics.chic_rt_arc_drop(& cloned);
                        Std.Memory.GlobalAllocator.Free(dest);
                        ThrowSpawn(status);
                    }
                    // Balance the temporary clone; the runtime now owns the context copy.
                    Std.Sync.RuntimeIntrinsics.chic_rt_arc_drop(& cloned);
                }
                return thread;
            }
            public ThreadStatus Join(ref this) {
                if (! _hasHandle)
                {
                    return ThreadStatus.Invalid;
                }
                var handle = _handle;
                var status = RuntimeExports.chic_rt_thread_join(ref handle);
                _handle = handle;
                _hasHandle = false;
                _completed = status == ThreadStatus.Success;
                return status;
            }
            public bool IsJoinable {
                get {
                    return _hasHandle;
                }
            }
            public void dispose(ref this) {
                if (_hasHandle)
                {
                    var handle = _handle;
                    var status = RuntimeExports.chic_rt_thread_detach(ref handle);
                    _handle = handle;
                    _hasHandle = false;
                    _handle.Clear();
                }
            }
            public ThreadStatus Detach(ref this) {
                if (! _hasHandle)
                {
                    return ThreadStatus.Invalid;
                }
                var handle = _handle;
                var status = RuntimeExports.chic_rt_thread_detach(ref handle);
                _handle = handle;
                _hasHandle = false;
                _handle.Clear();
                return status;
            }
            public static void Sleep(ulong milliseconds) {
                RuntimeExports.chic_rt_thread_sleep_ms(milliseconds);
            }
            public static void Yield() {
                RuntimeExports.chic_rt_thread_yield();
            }
            public static void SpinWait(uint iterations) {
                // Busy loop to simulate spin.
                var remaining = iterations;
                while (remaining >0)
                {
                    remaining -= 1;
                }
            }
            private static void ThrowSpawn(ThreadStatus status) {
                switch (status)
                {
                    case ThreadStatus.NotSupported:
                        throw new Std.InvalidOperationException(Std.Runtime.StringRuntime.FromStr("threads are not supported on this target"));
                    case ThreadStatus.SpawnFailed:
                        throw new Std.InvalidOperationException(Std.Runtime.StringRuntime.FromStr("failed to spawn thread"));
                    default :
                        throw new Std.InvalidOperationException(Std.Runtime.StringRuntime.FromStr("invalid thread spawn status"));
                    }
                }
                internal static void ValidateContextLayout(ValueMutPtr context) {
                    if (! RuntimeCallbacks.ContextLayoutIsValid (context))
                    {
                        throw new Std.InvalidOperationException(Std.Runtime.StringRuntime.FromStr("thread start context has invalid layout"));
                    }
                }
                private static Std.Sync.__StdSyncArcHandle AsArcHandle(ValueMutPtr context) {
                    ValidateContextLayout(context);
                    var handle = CoreIntrinsics.DefaultValue <Std.Sync.__StdSyncArcHandle >();
                    unsafe {
                        let dest = Std.Runtime.Collections.ValuePointer.CreateMut(Std.Numeric.PointerIntrinsics.AsByteMut(& handle),
                        (usize) __sizeof <Std.Sync.__StdSyncArcHandle >(), (usize) __alignof <Std.Sync.__StdSyncArcHandle >());
                        let source = Std.Runtime.Collections.ValuePointer.CreateConst(Std.Numeric.PointerIntrinsics.AsByteConstFromMut(context.Pointer),
                        context.Size, context.Alignment);
                        Std.Memory.GlobalAllocator.Copy(dest, source, dest.Size);
                    }
                    return handle;
                }
                private static string NormalizeName(string requested, int maxBytes, out ValueMutPtr buffer, out bool hasName) {
                    let name = requested == null ?"" : requested;
                    let utf8 = name.AsUtf8Span();
                    let maxAllowed = maxBytes <= 0 ?0usize : (usize) maxBytes;
                    let copyLen = utf8.Length <maxAllowed ?utf8.Length : maxAllowed;
                    if (copyLen == 0 || maxAllowed == 0)
                    {
                        buffer = Std.Runtime.Collections.ValuePointer.NullMut(1usize, 1usize);
                        hasName = false;
                        return "";
                    }
                    let allocSize = copyLen + 1usize;
                    var handle = Std.Runtime.Collections.ValuePointer.NullMut(1usize, 1usize);
                    let allocStatus = Std.Memory.GlobalAllocator.AllocZeroed(allocSize, 1usize, out handle);
                    if (allocStatus != AllocationError.Success)
                    {
                        buffer = Std.Runtime.Collections.ValuePointer.NullMut(1usize, 1usize);
                        hasName = false;
                        ThrowSpawn(ThreadStatus.SpawnFailed);
                    }
                    var destination = Std.Span.Span <byte >.FromValuePointer(handle, copyLen);
                    destination.CopyFrom(utf8.Slice(0usize, copyLen));
                    let safeLen = TrimUtf8Tail(handle, copyLen);
                    if (safeLen == 0)
                    {
                        Std.Memory.GlobalAllocator.Free(handle);
                        buffer = Std.Runtime.Collections.ValuePointer.NullMut(1usize, 1usize);
                        hasName = false;
                        return "";
                    }
                    if (safeLen <copyLen)
                    {
                        let tail = Std.Memory.GlobalAllocator.Offset(handle, (isize) safeLen);
                        Std.Memory.GlobalAllocator.Set(tail, 0, copyLen - safeLen);
                    }
                    buffer = handle;
                    hasName = true;
                    let nameHandle = Std.Runtime.Collections.ValuePointer.CreateConst(Std.Numeric.PointerIntrinsics.AsByteConstFromMut(handle.Pointer),
                    1usize, 1usize);
                    let nameSpan = Std.Span.ReadOnlySpan <byte >.FromValuePointer(nameHandle, safeLen);
                    let normalized = Std.Strings.Utf8String.FromSpan(nameSpan);
                    return normalized;
                }
                private static usize TrimUtf8Tail(ValueMutPtr data, usize length) {
                    if (length == 0 || Std.Runtime.Collections.ValuePointer.IsNullMut (data))
                    {
                        return 0usize;
                    }
                    var remaining = length;
                    unsafe {
                        while (remaining >0)
                        {
                            let index = remaining - 1usize;
                            let addr = (usize) Std.Numeric.Pointer.HandleFrom <byte >(data.Pointer) + index;
                            let currentPtr = (* const @readonly @expose_address byte) addr;
                            let current = * currentPtr;
                            if ( (current & 0xC0) == 0x80)
                            {
                                // Continuation byte; drop it and keep scanning backwards.
                                remaining -= 1usize;
                                continue;
                            }
                            let expected = Utf8SequenceLength(current);
                            if (expected == 0 || expected >remaining)
                            {
                                remaining = index;
                                continue;
                            }
                            break;
                        }
                    }
                    return remaining;
                }
                private static usize Utf8SequenceLength(byte lead) {
                    if ( (lead & 0x80) == 0)
                    {
                        return 1usize;
                    }
                    if ( (lead & 0xE0) == 0xC0)
                    {
                        return 2usize;
                    }
                    if ( (lead & 0xF0) == 0xE0)
                    {
                        return 3usize;
                    }
                    if ( (lead & 0xF8) == 0xF0)
                    {
                        return 4usize;
                    }
                    return 0usize;
                }
                }
