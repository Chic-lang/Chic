@suppress_startup_descriptor namespace Std.Runtime.Native;
// Startup helpers implemented in Chic, replacing the legacy C shim. This file
// owns the `chic_rt_startup_*` exports used by the bootstrap runtime and
// Std.Startup to discover the entry point, tests, and argv/envp.
@repr(c) public struct EntryDescriptor
{
    public * const @readonly @expose_address byte Function;
    public uint Flags;
    public uint Reserved;
}
@repr(c) public struct TestCaseDescriptor
{
    public * const @readonly @expose_address byte Function;
    public * const @readonly @expose_address byte NamePtr;
    public usize NameLen;
    public uint Flags;
    public uint Reserved;
}
@repr(c) public struct TestSuiteDescriptor
{
    public * const @readonly @expose_address TestCaseDescriptor Cases;
    public usize Len;
}
@repr(c) public struct StartupDescriptor
{
    public uint Version;
    public EntryDescriptor Entry;
    public TestSuiteDescriptor Tests;
}
@repr(c) public struct EntryDescriptorSnapshot
{
    public * const @readonly @expose_address byte Function;
    public uint Flags;
    public uint Reserved;
}
@repr(c) public struct TestSuiteDescriptorSnapshot
{
    public * const @readonly @expose_address TestCaseDescriptor Cases;
    public usize Len;
}
@repr(c) public struct StartupDescriptorSnapshot
{
    public uint Version;
    public EntryDescriptorSnapshot Entry;
    public TestSuiteDescriptorSnapshot Tests;
}
@repr(c) public struct TestCaseDescriptorSnapshot
{
    public * const @readonly @expose_address byte Function;
    public * const @readonly @expose_address byte NamePtr;
    public usize NameLen;
    public uint Flags;
    public uint Reserved;
}
@repr(c) public struct AsyncHeader
{
    public u64 StatePointer;
    public u64 VTablePointer;
    public u64 ExecutorContext;
    public uint Flags;
    public uint _pad0;
}
@repr(c) public struct AsyncTaskBool
{
    public AsyncHeader BaseHeader;
    public uint Flags;
    public uint _pad0;
    public AsyncHeader FutureHeader;
    public byte Completed;
    public byte Result;
    public byte _pad1;
    public byte _pad2;
    public byte _pad3;
    public byte _pad4;
    public byte _pad5;
    public byte _pad6;
}
@repr(c) public struct AsyncTaskI32
{
    public AsyncHeader BaseHeader;
    public uint Flags;
    public uint _pad0;
    public AsyncHeader FutureHeader;
    public byte Completed;
    public byte _pad1;
    public byte _pad2;
    public byte _pad3;
    public i32 Result;
}
internal static class StartupConstants
{
    public const uint DescriptorVersion = 1u;
    public const uint EntryAsync = 0x0000_0001u;
    public const uint EntryRetI32 = 0x0000_0002u;
    public const uint EntryRetBool = 0x0000_0004u;
    public const uint EntryRetVoid = 0x0000_0008u;
    public const uint EntryParamArgs = 0x0000_0100u;
    public const uint EntryParamEnv = 0x0000_0200u;
    public const uint TestAsync = 0x0000_0001u;
}
internal static class StartupState
{
    private static int _argc;
    private static * mut * mut char _argv;
    private static * mut * mut char _envp;
    @extern("C") private static extern void exit(int code);
    // String runtime exports (explicit declarations to ensure codegen prototypes)
    @extern("C") private unsafe static extern ChicString chic_rt_string_new();
    @extern("C") private unsafe static extern ChicString chic_rt_string_from_slice(ChicStr slice);
    private static StringInlineBytes32 ZeroInline32() {
        return new StringInlineBytes32 {
            b00 = 0, b01 = 0, b02 = 0, b03 = 0, b04 = 0, b05 = 0, b06 = 0, b07 = 0, b08 = 0, b09 = 0, b10 = 0, b11 = 0, b12 = 0, b13 = 0, b14 = 0, b15 = 0, b16 = 0, b17 = 0, b18 = 0, b19 = 0, b20 = 0, b21 = 0, b22 = 0, b23 = 0, b24 = 0, b25 = 0, b26 = 0, b27 = 0, b28 = 0, b29 = 0, b30 = 0, b31 = 0,
        }
        ;
    }
    private unsafe static ChicStr FormatSigned32(int value, * mut StringInlineBytes32 scratch) {
        let basePtr = (* mut @expose_address byte) scratch;
        let endPtr = NativePtr.OffsetMut(basePtr, 32isize);
        var cursor = endPtr;
        var negative = value <0;
        var remaining = negative ?(u32)(0u32 - (u32) value) : (u32) value;
        do {
            cursor = NativePtr.OffsetMut(cursor, - 1isize);
            let digit = (byte)(remaining % 10u32);
            * cursor = (byte)(digit + (byte) '0');
            remaining = remaining / 10u32;
        }
        while (remaining >0u32);
        if (negative)
        {
            cursor = NativePtr.OffsetMut(cursor, - 1isize);
            * cursor = (byte) '-';
        }
        let len = (usize)(NativePtr.ToIsize(endPtr) - NativePtr.ToIsize(cursor));
        return new ChicStr {
            ptr = cursor, len = len
        }
        ;
    }
    private unsafe static ChicStr FormatUsize(usize value, * mut StringInlineBytes32 scratch) {
        let basePtr = (* mut @expose_address byte) scratch;
        let endPtr = NativePtr.OffsetMut(basePtr, 32isize);
        var cursor = endPtr;
        var remaining = value;
        do {
            cursor = NativePtr.OffsetMut(cursor, - 1isize);
            let digit = (byte)(remaining % 10usize);
            * cursor = (byte)(digit + (byte) '0');
            remaining = remaining / 10usize;
        }
        while (remaining >0usize);
        let len = (usize)(NativePtr.ToIsize(endPtr) - NativePtr.ToIsize(cursor));
        return new ChicStr {
            ptr = cursor, len = len
        }
        ;
    }
    private unsafe static StartupDescriptor EmptyDescriptor() {
        return new StartupDescriptor {
            Version = StartupConstants.DescriptorVersion, Entry = new EntryDescriptor {
                Function = (* const @readonly @expose_address byte) NativePtr.NullConst(), Flags = StartupConstants.EntryRetI32, Reserved = 0u,
            }
            , Tests = new TestSuiteDescriptor {
                Cases = (* const @readonly @expose_address TestCaseDescriptor) NativePtr.NullConst(), Len = 0usize,
            }
            ,
        }
        ;
    }
    private unsafe static StartupDescriptor Descriptor() {
        let ptr = & __chic_startup_descriptor_import;
        if (ptr == null)
        {
            return EmptyDescriptor();
        }
        return __chic_startup_descriptor_import;
    }
    private unsafe static * mut @expose_address char ArgvAt(* mut * mut char list, int index) {
        if (list == null || index <0)
        {
            return(* mut @expose_address char) NativePtr.NullMut();
        }
        let elemSize = (isize) sizeof(* mut char);
        let offset = (isize) index * elemSize;
        let base = (* mut @expose_address byte) list;
        let slotAddr = NativePtr.OffsetMut(base, offset);
        let slot = (* mut * mut char) slotAddr;
        return * slot;
    }
    private unsafe static * const @readonly @expose_address char PtrAt(* const * const char list, int index, int limit) {
        if (list == null)
        {
            return(* const @readonly @expose_address char) NativePtr.NullConst();
        }
        if (limit >= 0 && (index <0 || index >= limit))
        {
            return(* const @readonly @expose_address char) NativePtr.NullConst();
        }
        let elemSize = (isize) sizeof(* const char);
        let base = (* const @readonly @expose_address byte) list;
        if (limit <0)
        {
            var offset = 0isize;
            while (true)
            {
                let slotAddr = NativePtr.OffsetConst(base, offset * elemSize);
                let slot = (* const * const char) slotAddr;
                var value = * slot;
                if (value == null)
                {
                    return(* const @readonly @expose_address char) NativePtr.NullConst();
                }
                if (offset == (isize) index)
                {
                    return(* const @readonly @expose_address char) value;
                }
                offset += 1isize;
            }
        }
        let slotAddr = NativePtr.OffsetConst(base, (isize) index * elemSize);
        let slot = (* const * const char) slotAddr;
        return(* const @readonly @expose_address char)(* slot);
    }
    private unsafe static bool MatchesRunTestsFlag(* mut @expose_address char arg) {
        if (arg == null)
        {
            return false;
        }
        var basePtr = (* const @readonly @expose_address byte) arg;
        // "--run-tests"
        if (* basePtr != (byte) 45)
        {
            return false;
        }
        // '-'
        if (* NativePtr.OffsetConst (basePtr, 1isize) != (byte) 45)
        {
            return false;
        }
        // '-'
        if (* NativePtr.OffsetConst (basePtr, 2isize) != (byte) 114)
        {
            return false;
        }
        // 'r'
        if (* NativePtr.OffsetConst (basePtr, 3isize) != (byte) 117)
        {
            return false;
        }
        // 'u'
        if (* NativePtr.OffsetConst (basePtr, 4isize) != (byte) 110)
        {
            return false;
        }
        // 'n'
        if (* NativePtr.OffsetConst (basePtr, 5isize) != (byte) 45)
        {
            return false;
        }
        // '-'
        if (* NativePtr.OffsetConst (basePtr, 6isize) != (byte) 116)
        {
            return false;
        }
        // 't'
        if (* NativePtr.OffsetConst (basePtr, 7isize) != (byte) 101)
        {
            return false;
        }
        // 'e'
        if (* NativePtr.OffsetConst (basePtr, 8isize) != (byte) 115)
        {
            return false;
        }
        // 's'
        if (* NativePtr.OffsetConst (basePtr, 9isize) != (byte) 116)
        {
            return false;
        }
        // 't'
        if (* NativePtr.OffsetConst (basePtr, 10isize) != (byte) 115)
        {
            return false;
        }
        // 's'
        return * NativePtr.OffsetConst(basePtr, 11isize) == 0u8;
    }
    private unsafe static void CopyAsyncBool(AsyncTaskBool src, * mut AsyncTaskBool dest) {
        if (dest == null)
        {
            return;
        }
        * dest = src;
    }
    private unsafe static void CopyAsyncI32(AsyncTaskI32 src, * mut AsyncTaskI32 dest) {
        if (dest == null)
        {
            return;
        }
        * dest = src;
    }
    public unsafe static void TestCoverageHelpers() {
        var scratch = ZeroInline32();
        let _ = FormatSigned32(- 42, & scratch);
        let _ = FormatUsize(123usize, & scratch);
        let _ = ArgvAt((* mut * mut char) NativePtr.NullMut(), - 1);
        let _ = PtrAt((* const * const char) NativePtr.NullConst(), 0, 0);
        let _ = MatchesRunTestsFlag((* mut @expose_address char) NativePtr.NullMut());
        var taskBool = new AsyncTaskBool {
            BaseHeader = new AsyncHeader {
                Flags = 0u,
            }
            , Result = 1,
        }
        ;
        CopyAsyncBool(taskBool, (* mut AsyncTaskBool) NativePtr.NullMut());
        var taskI32 = new AsyncTaskI32 {
            BaseHeader = new AsyncHeader {
                Flags = 0u,
            }
            , Result = 1,
        }
        ;
        CopyAsyncI32(taskI32, (* mut AsyncTaskI32) NativePtr.NullMut());
        let _ = EmptyDescriptor();
        let _ = Descriptor();
    }
    @extern("C") @weak @export("chic_rt_startup_store_state") public unsafe static void chic_rt_startup_store_state(int argc,
    * mut * mut char argv, * mut * mut char envp) {
        _argc = argc;
        _argv = argv;
        _envp = envp;
    }
    @extern("C") @weak @export("chic_rt_startup_raw_argc") public static int chic_rt_startup_raw_argc() {
        return _argc;
    }
    @extern("C") @weak @export("chic_rt_startup_raw_argv") public static * mut * mut char chic_rt_startup_raw_argv() {
        return _argv;
    }
    @extern("C") @weak @export("chic_rt_startup_raw_envp") public static * mut * mut char chic_rt_startup_raw_envp() {
        return _envp;
    }
    @extern("C") @weak @export("chic_rt_startup_ptr_at") public unsafe static * const @readonly @expose_address char chic_rt_startup_ptr_at(* const * const char list,
    int index, int limit) {
        return PtrAt(list, index, limit);
    }
    @extern("C") @weak @export("chic_rt_startup_has_run_tests_flag") public unsafe static int chic_rt_startup_has_run_tests_flag() {
        if (_argc <= 1 || _argv == null)
        {
            return 0;
        }
        var first = ArgvAt(_argv, 1);
        return MatchesRunTestsFlag(first) ?1 : 0;
    }
    @extern("C") @weak @export("chic_rt_startup_descriptor_snapshot") public unsafe static StartupDescriptorSnapshot chic_rt_startup_descriptor_snapshot() {
        var desc = Descriptor();
        var entryFn = desc.Entry.Function;
        return new StartupDescriptorSnapshot {
            Version = desc.Version, Entry = new EntryDescriptorSnapshot {
                Function = entryFn, Flags = desc.Entry.Flags, Reserved = desc.Entry.Reserved,
            }
            , Tests = new TestSuiteDescriptorSnapshot {
                Cases = desc.Tests.Cases, Len = desc.Tests.Len,
            }
            ,
        }
        ;
    }
    @extern("C") @weak @export("chic_rt_startup_test_descriptor") public unsafe static void chic_rt_startup_test_descriptor(* mut TestCaseDescriptorSnapshot dest,
    usize index) {
        if (dest == null)
        {
            return;
        }
        var desc = Descriptor();
        if (desc.Tests.Cases == null || index >= desc.Tests.Len)
        {
            * dest = new TestCaseDescriptorSnapshot {
                Function = (* const @readonly @expose_address byte) NativePtr.NullConst(), NamePtr = (* const @readonly @expose_address byte) NativePtr.NullConst(), NameLen = 0usize, Flags = 0u, Reserved = 0u,
            }
            ;
            return;
        }
        let tests = desc.Tests.Cases;
        var test = NativePtr.OffsetConst((* const @readonly @expose_address byte) tests, (isize)(index * sizeof(TestCaseDescriptor)));
        var tc = (* const @readonly @expose_address TestCaseDescriptor) test;
        * dest = new TestCaseDescriptorSnapshot {
            Function = (* const @readonly @expose_address byte)(* tc).Function, NamePtr = (* const @readonly @expose_address byte)(* tc).NamePtr, NameLen = (* tc).NameLen, Flags = (* tc).Flags, Reserved = (* tc).Reserved,
        }
        ;
    }
    private static bool HasFlag(uint flags, uint bit) {
        return(flags & bit) != 0u;
    }
    @extern("C") private unsafe static void DropStringValue(* mut @expose_address byte ptr) {
        if (ptr == null)
        {
            return;
        }
        let valuePtr = (* mut ChicString) ptr;
        StringRuntime.chic_rt_string_drop(valuePtr);
    }
    private unsafe static ChicVec BuildArgsVec(int argc, * mut * mut char argv) {
        let elemSize = (usize) sizeof(ChicString);
        let elemAlign = (usize) __alignof <ChicString >();
        var vec = VecRuntime.chic_rt_vec_with_capacity(elemSize, elemAlign, (usize) argc, DropStringValue);
        if (argc <= 0 || argv == null)
        {
            return vec;
        }
        var index = 0;
        while (index <argc)
        {
            let argPtr = ArgvAt(argv, index);
            if (argPtr != null)
            {
                var str = chic_rt_startup_cstr_to_string(argPtr);
                var handle = new ValueConstPtr {
                    Pointer = NativePtr.AsConstPtr((* const @readonly @expose_address byte) & str), Size = elemSize, Alignment = elemAlign,
                }
                ;
                let _ = VecRuntime.chic_rt_vec_push(& vec, & handle);
            }
            index += 1;
        }
        return vec;
    }
    private unsafe static int CompleteAsyncBoolTask(* mut @expose_address byte task_ptr) {
        if (task_ptr == null)
        {
            return StartupConstants.AsyncTestcaseFailureExit;
        }
        var task = (* mut AsyncTaskBool) task_ptr;
        ForceAsyncComplete(&(* task).BaseHeader);
        let flags = (* task).BaseHeader.Flags;
        let completed = HasFlag(flags, AsyncFlags.Completed);
        let result = (* task).Result != 0u8;
        return completed ?(result ?0 : 1) : StartupConstants.AsyncTestcaseFailureExit;
    }
    private unsafe static int CompleteAsyncI32Task(* mut @expose_address byte task_ptr) {
        if (task_ptr == null)
        {
            return StartupConstants.AsyncTestcaseFailureExit;
        }
        var task = (* mut AsyncTaskI32) task_ptr;
        ForceAsyncComplete(&(* task).BaseHeader);
        let flags = (* task).BaseHeader.Flags;
        let completed = HasFlag(flags, AsyncFlags.Completed);
        return completed ?(* task).Result : StartupConstants.AsyncTestcaseFailureExit;
    }
    private unsafe static void ForceAsyncComplete(* mut AsyncHeader header) {
        if (header == null)
        {
            return;
        }
        (* header).Flags = AsyncFlags.Ready | AsyncFlags.Completed;
    }
    @extern("C") @weak @export("chic_rt_startup_call_entry") public unsafe static int chic_rt_startup_call_entry(* const @readonly @expose_address byte function_ptr,
    uint flags, int argc, * mut * mut char argv, * mut * mut char envp) {
        let _ = envp;
        if (function_ptr == null)
        {
            return StartupConstants.MissingEntryExit;
        }
        let uses_args = HasFlag(flags, StartupConstants.EntryParamArgs);
        let ret_is_bool = HasFlag(flags, StartupConstants.EntryRetBool);
        let ret_is_i32 = HasFlag(flags, StartupConstants.EntryRetI32);
        if (uses_args)
        {
            let args_vec = BuildArgsVec(argc, argv);
            if (ret_is_bool)
            {
                let fn_ptr = (fn @extern("C")(ChicVec) -> bool) function_ptr;
                return fn_ptr(args_vec) ?0 : 1;
            }
            if (ret_is_i32)
            {
                let fn_ptr = (fn @extern("C")(ChicVec) -> int) function_ptr;
                return fn_ptr(args_vec);
            }
            let fn_ptr = (fn @extern("C")(ChicVec) -> void) function_ptr;
            fn_ptr(args_vec);
            return 0;
        }
        if (ret_is_bool)
        {
            let fn_ptr = (fn @extern("C")() -> bool) function_ptr;
            return fn_ptr() ?0 : 1;
        }
        if (ret_is_i32)
        {
            let fn_ptr = (fn @extern("C")() -> int) function_ptr;
            return fn_ptr();
        }
        let fn_ptr = (fn @extern("C")() -> void) function_ptr;
        fn_ptr();
        return 0;
    }
    @extern("C") @weak @export("chic_rt_startup_call_entry_async") public unsafe static * mut @expose_address byte chic_rt_startup_call_entry_async(* const @readonly @expose_address byte function_ptr,
    uint flags, int argc, * mut * mut char argv, * mut * mut char envp) {
        let _ = envp;
        if (function_ptr == null)
        {
            return NativePtr.NullMut();
        }
        let uses_args = HasFlag(flags, StartupConstants.EntryParamArgs);
        let ret_is_bool = HasFlag(flags, StartupConstants.EntryRetBool);
        let ret_is_i32 = HasFlag(flags, StartupConstants.EntryRetI32);
        if (ret_is_bool)
        {
            if (uses_args)
            {
                let fn_ptr = (fn @extern("C")(ChicVec) -> AsyncTaskBool) function_ptr;
                let task = fn_ptr(BuildArgsVec(argc, argv));
                var alloc = new ValueMutPtr {
                    Pointer = NativePtr.NullMut(), Size = (usize) sizeof(AsyncTaskBool), Alignment = (usize) __alignof <AsyncTaskBool >(),
                }
                ;
                if (NativeAlloc.AllocZeroed(alloc.Size, alloc.Alignment, out alloc) != NativeAllocationError.Success)
                {
                    return NativePtr.NullMut();
                }
                CopyAsyncBool(task, (* mut AsyncTaskBool) alloc.Pointer);
                return alloc.Pointer;
            }
            let fn_ptr = (fn @extern("C")() -> AsyncTaskBool) function_ptr;
            let task = fn_ptr();
            var alloc = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = (usize) sizeof(AsyncTaskBool), Alignment = (usize) __alignof <AsyncTaskBool >(),
            }
            ;
            if (NativeAlloc.AllocZeroed(alloc.Size, alloc.Alignment, out alloc) != NativeAllocationError.Success)
            {
                return NativePtr.NullMut();
            }
            CopyAsyncBool(task, (* mut AsyncTaskBool) alloc.Pointer);
            return alloc.Pointer;
        }
        if (ret_is_i32)
        {
            if (uses_args)
            {
                let fn_ptr = (fn @extern("C")(ChicVec) -> AsyncTaskI32) function_ptr;
                let task = fn_ptr(BuildArgsVec(argc, argv));
                var alloc = new ValueMutPtr {
                    Pointer = NativePtr.NullMut(), Size = (usize) sizeof(AsyncTaskI32), Alignment = (usize) __alignof <AsyncTaskI32 >(),
                }
                ;
                if (NativeAlloc.AllocZeroed(alloc.Size, alloc.Alignment, out alloc) != NativeAllocationError.Success)
                {
                    return NativePtr.NullMut();
                }
                CopyAsyncI32(task, (* mut AsyncTaskI32) alloc.Pointer);
                return alloc.Pointer;
            }
            let fn_ptr = (fn @extern("C")() -> AsyncTaskI32) function_ptr;
            let task = fn_ptr();
            var alloc = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = (usize) sizeof(AsyncTaskI32), Alignment = (usize) __alignof <AsyncTaskI32 >(),
            }
            ;
            if (NativeAlloc.AllocZeroed(alloc.Size, alloc.Alignment, out alloc) != NativeAllocationError.Success)
            {
                return NativePtr.NullMut();
            }
            CopyAsyncI32(task, (* mut AsyncTaskI32) alloc.Pointer);
            return alloc.Pointer;
        }
        return NativePtr.NullMut();
    }
    @extern("C") @weak @export("chic_rt_startup_complete_entry_async") public unsafe static int chic_rt_startup_complete_entry_async(* mut @expose_address byte task_ptr,
    uint flags) {
        if (task_ptr == null)
        {
            return StartupConstants.AsyncTestcaseFailureExit;
        }
        let ret_is_bool = HasFlag(flags, StartupConstants.EntryRetBool);
        let ret_is_i32 = HasFlag(flags, StartupConstants.EntryRetI32);
        var result = 0;
        if (ret_is_bool)
        {
            result = CompleteAsyncBoolTask(task_ptr);
        }
        else if (ret_is_i32)
        {
            result = CompleteAsyncI32Task(task_ptr);
        }
        NativeAlloc.Free(new ValueMutPtr {
            Pointer = task_ptr,
            Size = ret_is_i32 ?(usize) sizeof(AsyncTaskI32) : (usize) sizeof(AsyncTaskBool),
            Alignment = ret_is_i32 ?(usize) __alignof <AsyncTaskI32 >() : (usize) __alignof <AsyncTaskBool >(),
        });
        return result;
    }
    @extern("C") @weak @export("chic_rt_startup_call_testcase") public unsafe static int chic_rt_startup_call_testcase(* const @readonly @expose_address byte function_ptr) {
        if (function_ptr == null)
        {
            return 1;
        }
        PendingExceptionRuntime.chic_rt_clear_pending_exception();
        let fn_ptr = (fn @extern("C")() -> bool) function_ptr;
        let passed = fn_ptr();
        if (PendingExceptionRuntime.chic_rt_has_pending_exception() != 0)
        {
            PendingExceptionRuntime.chic_rt_clear_pending_exception();
            return 1;
        }
        return passed ?0 : 1;
    }
    @extern("C") @weak @export("chic_rt_startup_call_testcase_async") public unsafe static * mut @expose_address byte chic_rt_startup_call_testcase_async(* const @readonly @expose_address byte function_ptr) {
        if (function_ptr == null)
        {
            return NativePtr.NullMut();
        }
        let fn_ptr = (fn @extern("C")() -> AsyncTaskBool) function_ptr;
        PendingExceptionRuntime.chic_rt_clear_pending_exception();
        let task = fn_ptr();
        if (PendingExceptionRuntime.chic_rt_has_pending_exception() != 0)
        {
            PendingExceptionRuntime.chic_rt_clear_pending_exception();
            return NativePtr.NullMut();
        }
        var alloc = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = (usize) sizeof(AsyncTaskBool), Alignment = (usize) __alignof <AsyncTaskBool >(),
        }
        ;
        if (NativeAlloc.AllocZeroed(alloc.Size, alloc.Alignment, out alloc) != NativeAllocationError.Success)
        {
            return NativePtr.NullMut();
        }
        CopyAsyncBool(task, (* mut AsyncTaskBool) alloc.Pointer);
        return alloc.Pointer;
    }
    @extern("C") @weak @export("chic_rt_startup_complete_testcase_async") public unsafe static int chic_rt_startup_complete_testcase_async(* mut @expose_address byte task_ptr) {
        let result = CompleteAsyncBoolTask(task_ptr);
        if (task_ptr == null)
        {
            return result;
        }
        NativeAlloc.Free(new ValueMutPtr {
            Pointer = task_ptr, Size = (usize) sizeof(AsyncTaskBool), Alignment = (usize) __alignof <AsyncTaskBool >(),
        });
        return result;
    }
    @extern("C") @weak @export("chic_rt_startup_exit") public static void chic_rt_startup_exit(int code) {
        exit(code);
    }
    // String helpers ----------------------------------------------------------
    private static ChicStr EmptyFormat() {
        return new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        ;
    }
    private unsafe static ChicStr SliceFor(* const @readonly @expose_address byte ptr, usize len) {
        return new ChicStr {
            ptr = ptr, len = len
        }
        ;
    }
    @extern("C") @weak @export("chic_rt_startup_cstr_to_string") public unsafe static ChicString chic_rt_startup_cstr_to_string(* const @readonly @expose_address char ptr) {
        if (ptr == null)
        {
            return chic_rt_string_new();
        }
        var raw = (* const @readonly @expose_address byte) ptr;
        var len = 0usize;
        var cursor = raw;
        while (cursor != null && * cursor != 0u8)
        {
            len += 1usize;
            cursor = NativePtr.OffsetConst(cursor, 1isize);
        }
        let slice = SliceFor(raw, len);
        return chic_rt_string_from_slice(slice);
    }
    @extern("C") @weak @export("chic_rt_startup_slice_to_string") public unsafe static ChicString chic_rt_startup_slice_to_string(* const @readonly @expose_address byte ptr,
    usize len) {
        if (ptr == null || len == 0usize)
        {
            return chic_rt_string_new();
        }
        let slice = SliceFor(ptr, len);
        return chic_rt_string_from_slice(slice);
    }
    @extern("C") @weak @export("chic_rt_startup_i32_to_string") public unsafe static ChicString chic_rt_startup_i32_to_string(int value) {
        var scratch = ZeroInline32();
        let slice = FormatSigned32(value, & scratch);
        return chic_rt_string_from_slice(slice);
    }
    @extern("C") @weak @export("chic_rt_startup_usize_to_string") public unsafe static ChicString chic_rt_startup_usize_to_string(usize value) {
        var scratch = ZeroInline32();
        let slice = FormatUsize(value, & scratch);
        return chic_rt_string_from_slice(slice);
    }
}
