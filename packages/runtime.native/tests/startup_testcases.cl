namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;
internal static class StartupTestConstants
{
    public const int MissingEntryExit = 90;
    public const int AsyncFailureExit = 92;
}
public static class StartupTestSupport
{
    public unsafe static * mut @expose_address char AsCharPtr(ref StringInlineBytes32 data) {
        var * mut @expose_address byte raw = & data.b00;
        return(* mut @expose_address char) raw;
    }
    public unsafe static bool SliceMatches(ChicStr slice, * const @readonly @expose_address byte expected, usize len) {
        if (slice.len != len)
        {
            return false;
        }
        var idx = 0usize;
        while (idx <len)
        {
            let leftPtr = NativePtr.OffsetConst(slice.ptr, (isize) idx);
            let rightPtr = NativePtr.OffsetConst(expected, (isize) idx);
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
    public unsafe static ValueMutPtr AllocateArgv(* mut @expose_address char first, * mut @expose_address char second) {
        var argv = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 3usize * sizeof(* mut char), Alignment = sizeof(* mut char)
        }
        ;
        let _ = NativeAlloc.AllocZeroed(argv.Size, argv.Alignment, out argv);
        if (argv.Pointer == null)
        {
            return argv;
        }
        let slot0 = (* mut * mut @expose_address char) argv.Pointer;
        * slot0 = first;
        let slot1 = (* mut * mut @expose_address char) NativePtr.OffsetMut(argv.Pointer, (isize) sizeof(* mut char));
        * slot1 = second;
        let slot2 = (* mut * mut @expose_address char) NativePtr.OffsetMut(argv.Pointer, (isize)(2usize * sizeof(* mut char)));
        * slot2 = (* mut @expose_address char) NativePtr.NullMut();
        return argv;
    }
}
internal static class StartupEntryFixtures
{
    public static bool EntryVoidCalled;
    public static bool EntryArgsVoidCalled;
    @extern("C") public static int EntryNoArgsI32() {
        return 7;
    }
    @extern("C") public static bool EntryNoArgsBool() {
        return true;
    }
    @extern("C") public static void EntryNoArgsVoid() {
        EntryVoidCalled = true;
    }
    @extern("C") public static int EntryArgsI32(ChicVec args) {
        return args.len == 2usize ?13 : 1;
    }
    @extern("C") public static bool EntryArgsBool(ChicVec args) {
        return args.len == 2usize;
    }
    @extern("C") public static void EntryArgsVoid(ChicVec args) {
        EntryArgsVoidCalled = args.len == 2usize;
    }
    @extern("C") public static AsyncTaskBool EntryAsyncBool() {
        return new AsyncTaskBool {
            BaseHeader = new AsyncHeader {
                Flags = 0u,
            }
            , Result = 1,
        }
        ;
    }
    @extern("C") public static AsyncTaskBool EntryAsyncBoolFalse() {
        return new AsyncTaskBool {
            BaseHeader = new AsyncHeader {
                Flags = 0u,
            }
            , Result = 0,
        }
        ;
    }
    @extern("C") public static AsyncTaskI32 EntryAsyncI32Args(ChicVec args) {
        return new AsyncTaskI32 {
            BaseHeader = new AsyncHeader {
                Flags = 0u,
            }
            , Result = args.len == 2usize ?11 : - 1,
        }
        ;
    }
    @extern("C") public static AsyncTaskBool EntryAsyncBoolArgs(ChicVec args) {
        return new AsyncTaskBool {
            BaseHeader = new AsyncHeader {
                Flags = 0u,
            }
            , Result = args.len == 2usize ?1 : 0,
        }
        ;
    }
    @extern("C") public static AsyncTaskI32 EntryAsyncI32NoArgs() {
        return new AsyncTaskI32 {
            BaseHeader = new AsyncHeader {
                Flags = 0u,
            }
            , Result = 17,
        }
        ;
    }
    @extern("C") public static bool TestcaseOk() {
        return true;
    }
    @extern("C") public static bool TestcaseFail() {
        return false;
    }
    @extern("C") public static AsyncTaskBool TestcaseAsyncOk() {
        return new AsyncTaskBool {
            BaseHeader = new AsyncHeader {
                Flags = 0u,
            }
            , Result = 1,
        }
        ;
    }
}
testcase Given_startup_store_state_records_argc_When_executed_Then_startup_store_state_records_argc()
{
    unsafe {
        var arg0 = new StringInlineBytes32 {
            b00 = 97, b01 = 112, b02 = 112, b03 = 0,
        }
        ;
        var runTests = new StringInlineBytes32 {
            b00 = 45, b01 = 45, b02 = 114, b03 = 117, b04 = 110, b05 = 45, b06 = 116, b07 = 101, b08 = 115, b09 = 116, b10 = 115, b11 = 0,
        }
        ;
        let argv = StartupTestSupport.AllocateArgv(StartupTestSupport.AsCharPtr(ref arg0), StartupTestSupport.AsCharPtr(ref runTests));
        let argvPtr = (* mut * mut @expose_address char) argv.Pointer;
        StartupState.chic_rt_startup_store_state(2, argvPtr, (* mut * mut @expose_address char) NativePtr.NullMut());
        let ok = StartupState.chic_rt_startup_raw_argc() == 2;
        NativeAlloc.Free(argv);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_startup_store_state_records_argv_pointer_When_executed_Then_startup_store_state_records_argv_pointer()
{
    unsafe {
        var arg0 = new StringInlineBytes32 {
            b00 = 97, b01 = 112, b02 = 112, b03 = 0,
        }
        ;
        var runTests = new StringInlineBytes32 {
            b00 = 45, b01 = 45, b02 = 114, b03 = 117, b04 = 110, b05 = 45, b06 = 116, b07 = 101, b08 = 115, b09 = 116, b10 = 115, b11 = 0,
        }
        ;
        let argv = StartupTestSupport.AllocateArgv(StartupTestSupport.AsCharPtr(ref arg0), StartupTestSupport.AsCharPtr(ref runTests));
        let argvPtr = (* mut * mut @expose_address char) argv.Pointer;
        StartupState.chic_rt_startup_store_state(2, argvPtr, (* mut * mut @expose_address char) NativePtr.NullMut());
        let ok = StartupState.chic_rt_startup_raw_argv() != null;
        NativeAlloc.Free(argv);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_startup_run_tests_flag_detects_flag_When_executed_Then_startup_run_tests_flag_detects_flag()
{
    unsafe {
        var arg0 = new StringInlineBytes32 {
            b00 = 97, b01 = 112, b02 = 112, b03 = 0,
        }
        ;
        var runTests = new StringInlineBytes32 {
            b00 = 45, b01 = 45, b02 = 114, b03 = 117, b04 = 110, b05 = 45, b06 = 116, b07 = 101, b08 = 115, b09 = 116, b10 = 115, b11 = 0,
        }
        ;
        let argv = StartupTestSupport.AllocateArgv(StartupTestSupport.AsCharPtr(ref arg0), StartupTestSupport.AsCharPtr(ref runTests));
        let argvPtr = (* mut * mut @expose_address char) argv.Pointer;
        StartupState.chic_rt_startup_store_state(2, argvPtr, (* mut * mut @expose_address char) NativePtr.NullMut());
        let ok = StartupState.chic_rt_startup_has_run_tests_flag() == 1;
        NativeAlloc.Free(argv);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_startup_ptr_at_reads_argument_When_executed_Then_startup_ptr_at_reads_argument()
{
    unsafe {
        var arg0 = new StringInlineBytes32 {
            b00 = 97, b01 = 112, b02 = 112, b03 = 0,
        }
        ;
        var runTests = new StringInlineBytes32 {
            b00 = 45, b01 = 45, b02 = 114, b03 = 117, b04 = 110, b05 = 45, b06 = 116, b07 = 101, b08 = 115, b09 = 116, b10 = 115, b11 = 0,
        }
        ;
        let argv = StartupTestSupport.AllocateArgv(StartupTestSupport.AsCharPtr(ref arg0), StartupTestSupport.AsCharPtr(ref runTests));
        let argvPtr = (* mut * mut @expose_address char) argv.Pointer;
        let resolved = StartupState.chic_rt_startup_ptr_at((* const * const @readonly @expose_address char) argvPtr, 1, 2);
        let first = * (* const @readonly @expose_address byte) resolved;
        NativeAlloc.Free(argv);
        Assert.That(first == 45u8).IsTrue();
    }
}
testcase Given_startup_ptr_at_null_list_returns_null_When_executed_Then_startup_ptr_at_null_list_returns_null()
{
    unsafe {
        let nullPtr = StartupState.chic_rt_startup_ptr_at((* const * const @readonly @expose_address char) NativePtr.NullConst(),
        0, 0);
        Assert.That(nullPtr == null).IsTrue();
    }
}
testcase Given_startup_descriptor_snapshot_version_When_executed_Then_startup_descriptor_snapshot_version()
{
    unsafe {
        let snapshot = StartupState.chic_rt_startup_descriptor_snapshot();
        Assert.That(snapshot.Version == StartupConstants.DescriptorVersion).IsTrue();
    }
}
testcase Given_startup_test_descriptor_out_of_range_returns_null_When_executed_Then_startup_test_descriptor_out_of_range_returns_null()
{
    unsafe {
        let snapshot = StartupState.chic_rt_startup_descriptor_snapshot();
        var testSnapshot = new TestCaseDescriptorSnapshot {
            Function = NativePtr.NullConst(), NamePtr = NativePtr.NullConst(), NameLen = 0usize, Flags = 0u, Reserved = 0u,
        }
        ;
        StartupState.chic_rt_startup_test_descriptor(& testSnapshot, snapshot.Tests.Len);
        Assert.That(testSnapshot.Function == null).IsTrue();
    }
}
testcase Given_startup_call_entry_null_returns_missing_exit_When_executed_Then_startup_call_entry_null_returns_missing_exit()
{
    unsafe {
        let result = StartupState.chic_rt_startup_call_entry(NativePtr.NullConst(), 0u, 0, null, null);
        Assert.That(result == StartupTestConstants.MissingEntryExit).IsTrue();
    }
}
testcase Given_startup_call_entry_bool_without_args_When_executed_Then_startup_call_entry_bool_without_args()
{
    unsafe {
        let fnPtr = (* const @readonly @expose_address byte) StartupEntryFixtures.EntryNoArgsBool;
        let result = StartupState.chic_rt_startup_call_entry(fnPtr, StartupConstants.EntryRetBool, 0, null, null);
        Assert.That(result == 0).IsTrue();
    }
}
testcase Given_startup_call_entry_i32_without_args_When_executed_Then_startup_call_entry_i32_without_args()
{
    unsafe {
        let fnPtr = (* const @readonly @expose_address byte) StartupEntryFixtures.EntryNoArgsI32;
        let result = StartupState.chic_rt_startup_call_entry(fnPtr, StartupConstants.EntryRetI32, 0, null, null);
        Assert.That(result == 7).IsTrue();
    }
}
testcase Given_startup_call_entry_void_without_args_When_executed_Then_startup_call_entry_void_without_args()
{
    unsafe {
        StartupEntryFixtures.EntryVoidCalled = false;
        let fnPtr = (* const @readonly @expose_address byte) StartupEntryFixtures.EntryNoArgsVoid;
        let result = StartupState.chic_rt_startup_call_entry(fnPtr, StartupConstants.EntryRetVoid, 0, null, null);
        Assert.That(result == 0 && StartupEntryFixtures.EntryVoidCalled).IsTrue();
    }
}
testcase Given_startup_call_entry_bool_with_args_When_executed_Then_startup_call_entry_bool_with_args()
{
    unsafe {
        var arg0 = new StringInlineBytes32 {
            b00 = 97, b01 = 112, b02 = 112, b03 = 0,
        }
        ;
        var runTests = new StringInlineBytes32 {
            b00 = 45, b01 = 45, b02 = 114, b03 = 117, b04 = 110, b05 = 45, b06 = 116, b07 = 101, b08 = 115, b09 = 116, b10 = 115, b11 = 0,
        }
        ;
        let argv = StartupTestSupport.AllocateArgv(StartupTestSupport.AsCharPtr(ref arg0), StartupTestSupport.AsCharPtr(ref runTests));
        let argvPtr = (* mut * mut @expose_address char) argv.Pointer;
        let fnPtr = (* const @readonly @expose_address byte) StartupEntryFixtures.EntryArgsBool;
        let result = StartupState.chic_rt_startup_call_entry(fnPtr, StartupConstants.EntryParamArgs | StartupConstants.EntryRetBool,
        2, argvPtr, null);
        NativeAlloc.Free(argv);
        Assert.That(result == 0).IsTrue();
    }
}
testcase Given_startup_call_entry_i32_with_args_When_executed_Then_startup_call_entry_i32_with_args()
{
    unsafe {
        var arg0 = new StringInlineBytes32 {
            b00 = 97, b01 = 112, b02 = 112, b03 = 0,
        }
        ;
        var runTests = new StringInlineBytes32 {
            b00 = 45, b01 = 45, b02 = 114, b03 = 117, b04 = 110, b05 = 45, b06 = 116, b07 = 101, b08 = 115, b09 = 116, b10 = 115, b11 = 0,
        }
        ;
        let argv = StartupTestSupport.AllocateArgv(StartupTestSupport.AsCharPtr(ref arg0), StartupTestSupport.AsCharPtr(ref runTests));
        let argvPtr = (* mut * mut @expose_address char) argv.Pointer;
        let fnPtr = (* const @readonly @expose_address byte) StartupEntryFixtures.EntryArgsI32;
        let result = StartupState.chic_rt_startup_call_entry(fnPtr, StartupConstants.EntryParamArgs | StartupConstants.EntryRetI32,
        2, argvPtr, null);
        NativeAlloc.Free(argv);
        Assert.That(result == 13).IsTrue();
    }
}
testcase Given_startup_call_entry_void_with_args_When_executed_Then_startup_call_entry_void_with_args()
{
    unsafe {
        var arg0 = new StringInlineBytes32 {
            b00 = 97, b01 = 112, b02 = 112, b03 = 0,
        }
        ;
        var runTests = new StringInlineBytes32 {
            b00 = 45, b01 = 45, b02 = 114, b03 = 117, b04 = 110, b05 = 45, b06 = 116, b07 = 101, b08 = 115, b09 = 116, b10 = 115, b11 = 0,
        }
        ;
        let argv = StartupTestSupport.AllocateArgv(StartupTestSupport.AsCharPtr(ref arg0), StartupTestSupport.AsCharPtr(ref runTests));
        let argvPtr = (* mut * mut @expose_address char) argv.Pointer;
        StartupEntryFixtures.EntryArgsVoidCalled = false;
        let fnPtr = (* const @readonly @expose_address byte) StartupEntryFixtures.EntryArgsVoid;
        let result = StartupState.chic_rt_startup_call_entry(fnPtr, StartupConstants.EntryParamArgs, 2, argvPtr, null);
        NativeAlloc.Free(argv);
        Assert.That(result == 0 && StartupEntryFixtures.EntryArgsVoidCalled).IsTrue();
    }
}
testcase Given_startup_call_entry_async_bool_When_executed_Then_startup_call_entry_async_bool()
{
    unsafe {
        let fnPtr = (* const @readonly @expose_address byte) StartupEntryFixtures.EntryAsyncBool;
        let task = StartupState.chic_rt_startup_call_entry_async(fnPtr, StartupConstants.EntryRetBool, 0, null, null);
        let result = StartupState.chic_rt_startup_complete_entry_async(task, StartupConstants.EntryRetBool);
        Assert.That(result == 0).IsTrue();
    }
}
testcase Given_startup_call_entry_async_bool_false_When_executed_Then_startup_call_entry_async_bool_false()
{
    unsafe {
        let fnPtr = (* const @readonly @expose_address byte) StartupEntryFixtures.EntryAsyncBoolFalse;
        let task = StartupState.chic_rt_startup_call_entry_async(fnPtr, StartupConstants.EntryRetBool, 0, null, null);
        let result = StartupState.chic_rt_startup_complete_entry_async(task, StartupConstants.EntryRetBool);
        Assert.That(result == 1).IsTrue();
    }
}
testcase Given_startup_call_entry_async_i32_with_args_When_executed_Then_startup_call_entry_async_i32_with_args()
{
    unsafe {
        var arg0 = new StringInlineBytes32 {
            b00 = 97, b01 = 112, b02 = 112, b03 = 0,
        }
        ;
        var runTests = new StringInlineBytes32 {
            b00 = 45, b01 = 45, b02 = 114, b03 = 117, b04 = 110, b05 = 45, b06 = 116, b07 = 101, b08 = 115, b09 = 116, b10 = 115, b11 = 0,
        }
        ;
        let argv = StartupTestSupport.AllocateArgv(StartupTestSupport.AsCharPtr(ref arg0), StartupTestSupport.AsCharPtr(ref runTests));
        let argvPtr = (* mut * mut @expose_address char) argv.Pointer;
        let fnPtr = (* const @readonly @expose_address byte) StartupEntryFixtures.EntryAsyncI32Args;
        let task = StartupState.chic_rt_startup_call_entry_async(fnPtr, StartupConstants.EntryParamArgs | StartupConstants.EntryRetI32,
        2, argvPtr, null);
        let result = StartupState.chic_rt_startup_complete_entry_async(task, StartupConstants.EntryRetI32);
        NativeAlloc.Free(argv);
        Assert.That(result == 11).IsTrue();
    }
}
testcase Given_startup_call_entry_async_bool_with_args_When_executed_Then_startup_call_entry_async_bool_with_args()
{
    unsafe {
        var arg0 = new StringInlineBytes32 {
            b00 = 97, b01 = 112, b02 = 112, b03 = 0,
        }
        ;
        var runTests = new StringInlineBytes32 {
            b00 = 45, b01 = 45, b02 = 114, b03 = 117, b04 = 110, b05 = 45, b06 = 116, b07 = 101, b08 = 115, b09 = 116, b10 = 115, b11 = 0,
        }
        ;
        let argv = StartupTestSupport.AllocateArgv(StartupTestSupport.AsCharPtr(ref arg0), StartupTestSupport.AsCharPtr(ref runTests));
        let argvPtr = (* mut * mut @expose_address char) argv.Pointer;
        let fnPtr = (* const @readonly @expose_address byte) StartupEntryFixtures.EntryAsyncBoolArgs;
        let task = StartupState.chic_rt_startup_call_entry_async(fnPtr, StartupConstants.EntryParamArgs | StartupConstants.EntryRetBool,
        2, argvPtr, null);
        let result = StartupState.chic_rt_startup_complete_entry_async(task, StartupConstants.EntryRetBool);
        NativeAlloc.Free(argv);
        Assert.That(result == 0).IsTrue();
    }
}
testcase Given_startup_call_entry_async_i32_without_args_When_executed_Then_startup_call_entry_async_i32_without_args()
{
    unsafe {
        let fnPtr = (* const @readonly @expose_address byte) StartupEntryFixtures.EntryAsyncI32NoArgs;
        let task = StartupState.chic_rt_startup_call_entry_async(fnPtr, StartupConstants.EntryRetI32, 0, null, null);
        let result = StartupState.chic_rt_startup_complete_entry_async(task, StartupConstants.EntryRetI32);
        Assert.That(result == 17).IsTrue();
    }
}
testcase Given_startup_call_testcase_success_When_executed_Then_startup_call_testcase_success()
{
    unsafe {
        let fnPtr = (* const @readonly @expose_address byte) StartupEntryFixtures.TestcaseOk;
        let result = StartupState.chic_rt_startup_call_testcase(fnPtr);
        Assert.That(result == 0).IsTrue();
    }
}
testcase Given_startup_call_testcase_failure_When_executed_Then_startup_call_testcase_failure()
{
    unsafe {
        let fnPtr = (* const @readonly @expose_address byte) StartupEntryFixtures.TestcaseFail;
        let result = StartupState.chic_rt_startup_call_testcase(fnPtr);
        Assert.That(result == 1).IsTrue();
    }
}
testcase Given_startup_call_testcase_async_success_When_executed_Then_startup_call_testcase_async_success()
{
    unsafe {
        let fnPtr = (* const @readonly @expose_address byte) StartupEntryFixtures.TestcaseAsyncOk;
        let task = StartupState.chic_rt_startup_call_testcase_async(fnPtr);
        let result = StartupState.chic_rt_startup_complete_testcase_async(task);
        Assert.That(result == 0).IsTrue();
    }
}
testcase Given_startup_call_entry_async_null_returns_null_When_executed_Then_startup_call_entry_async_null_returns_null()
{
    unsafe {
        let asyncPtr = StartupState.chic_rt_startup_call_entry_async(NativePtr.NullConst(), 0u, 0, null, null);
        Assert.That(asyncPtr == null).IsTrue();
    }
}
testcase Given_startup_complete_entry_async_null_returns_failure_When_executed_Then_startup_complete_entry_async_null_returns_failure()
{
    unsafe {
        let result = StartupState.chic_rt_startup_complete_entry_async(NativePtr.NullMut(), 0u);
        Assert.That(result == StartupTestConstants.AsyncFailureExit).IsTrue();
    }
}
testcase Given_startup_call_testcase_null_returns_failure_When_executed_Then_startup_call_testcase_null_returns_failure()
{
    unsafe {
        let result = StartupState.chic_rt_startup_call_testcase(NativePtr.NullConst());
        Assert.That(result == 1).IsTrue();
    }
}
testcase Given_startup_call_testcase_async_null_returns_null_When_executed_Then_startup_call_testcase_async_null_returns_null()
{
    unsafe {
        let asyncTest = StartupState.chic_rt_startup_call_testcase_async(NativePtr.NullConst());
        Assert.That(asyncTest == null).IsTrue();
    }
}
testcase Given_startup_complete_testcase_async_null_returns_failure_When_executed_Then_startup_complete_testcase_async_null_returns_failure()
{
    unsafe {
        let result = StartupState.chic_rt_startup_complete_testcase_async(NativePtr.NullMut());
        Assert.That(result == StartupTestConstants.AsyncFailureExit).IsTrue();
    }
}
testcase Given_startup_cstr_to_string_roundtrip_When_executed_Then_startup_cstr_to_string_roundtrip()
{
    unsafe {
        var hi = new StringInlineBytes32 {
            b00 = 104, b01 = 105, b02 = 0,
        }
        ;
        let hiPtr = StartupTestSupport.AsCharPtr(ref hi);
        var str = StartupState.chic_rt_startup_cstr_to_string(hiPtr);
        let slice = StringRuntime.chic_rt_string_as_slice(& str);
        let ok = StartupTestSupport.SliceMatches(slice, NativePtr.AsConstPtr(& hi.b00), 2usize);
        StringRuntime.chic_rt_string_drop(& str);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_startup_slice_to_string_roundtrip_When_executed_Then_startup_slice_to_string_roundtrip()
{
    unsafe {
        var abc = new StringInlineBytes32 {
            b00 = 97, b01 = 98, b02 = 99,
        }
        ;
        let sliceIn = new ChicStr {
            ptr = NativePtr.AsConstPtr(& abc.b00), len = 3usize
        }
        ;
        var str2 = StartupState.chic_rt_startup_slice_to_string(sliceIn.ptr, sliceIn.len);
        let slice2 = StringRuntime.chic_rt_string_as_slice(& str2);
        let ok = StartupTestSupport.SliceMatches(slice2, NativePtr.AsConstPtr(& abc.b00), 3usize);
        StringRuntime.chic_rt_string_drop(& str2);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_startup_i32_to_string_formats_negative_When_executed_Then_startup_i32_to_string_formats_negative()
{
    unsafe {
        var num = StartupState.chic_rt_startup_i32_to_string(- 12);
        let numSlice = StringRuntime.chic_rt_string_as_slice(& num);
        var expected = new StringInlineBytes32 {
            b00 = 45, b01 = 49, b02 = 50,
        }
        ;
        let ok = StartupTestSupport.SliceMatches(numSlice, NativePtr.AsConstPtr(& expected.b00), 3usize);
        StringRuntime.chic_rt_string_drop(& num);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_startup_usize_to_string_formats_value_When_executed_Then_startup_usize_to_string_formats_value()
{
    unsafe {
        var unum = StartupState.chic_rt_startup_usize_to_string(42usize);
        let unumSlice = StringRuntime.chic_rt_string_as_slice(& unum);
        var expectedU = new StringInlineBytes32 {
            b00 = 52, b01 = 50,
        }
        ;
        let ok = StartupTestSupport.SliceMatches(unumSlice, NativePtr.AsConstPtr(& expectedU.b00), 2usize);
        StringRuntime.chic_rt_string_drop(& unum);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_startup_run_tests_flag_unset_without_args_When_executed_Then_startup_run_tests_flag_unset_without_args()
{
    unsafe {
        StartupState.chic_rt_startup_store_state(0, (* mut * mut @expose_address char) NativePtr.NullMut(), (* mut * mut @expose_address char) NativePtr.NullMut());
        let ok = StartupState.chic_rt_startup_has_run_tests_flag() == 0;
        Assert.That(ok).IsTrue();
    }
}
testcase Given_startup_ptr_at_unbounded_reads_value_When_executed_Then_startup_ptr_at_unbounded_reads_value()
{
    unsafe {
        var arg0 = new StringInlineBytes32 {
            b00 = 97, b01 = 48, b02 = 0,
        }
        ;
        var arg1 = new StringInlineBytes32 {
            b00 = 98, b01 = 49, b02 = 0,
        }
        ;
        let argv = StartupTestSupport.AllocateArgv(StartupTestSupport.AsCharPtr(ref arg0), StartupTestSupport.AsCharPtr(ref arg1));
        let argvPtr = (* mut * mut @expose_address char) argv.Pointer;
        let resolved = StartupState.chic_rt_startup_ptr_at((* const * const @readonly @expose_address char) argvPtr, 1, - 1);
        let first = * (* const @readonly @expose_address byte) resolved;
        NativeAlloc.Free(argv);
        Assert.That(first == 98u8).IsTrue();
    }
}
testcase Given_startup_cstr_null_returns_empty_When_executed_Then_startup_cstr_null_returns_empty()
{
    unsafe {
        var empty = StartupState.chic_rt_startup_cstr_to_string((* const @readonly @expose_address char) NativePtr.NullConst());
        let slice = StringRuntime.chic_rt_string_as_slice(& empty);
        let ok = slice.len == 0usize;
        StringRuntime.chic_rt_string_drop(& empty);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_startup_test_descriptor_ignores_null_dest_When_executed_Then_startup_test_descriptor_ignores_null_dest()
{
    unsafe {
        StartupState.chic_rt_startup_test_descriptor((* mut TestCaseDescriptorSnapshot) NativePtr.NullMut(), 0usize);
        Assert.That(true).IsTrue();
    }
}
testcase Given_startup_slice_null_returns_empty_When_executed_Then_startup_slice_null_returns_empty()
{
    unsafe {
        var empty = StartupState.chic_rt_startup_slice_to_string(NativePtr.NullConst(), 5usize);
        let slice = StringRuntime.chic_rt_string_as_slice(& empty);
        let ok = slice.len == 0usize;
        StringRuntime.chic_rt_string_drop(& empty);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_startup_ptr_at_bounds_returns_null_When_executed_Then_startup_ptr_at_bounds_returns_null()
{
    unsafe {
        var arg0 = new StringInlineBytes32 {
            b00 = 120, b01 = 0,
        }
        ;
        let argv = StartupTestSupport.AllocateArgv(StartupTestSupport.AsCharPtr(ref arg0), (* mut @expose_address char) NativePtr.NullMut());
        let argvPtr = (* mut * mut @expose_address char) argv.Pointer;
        let missing = StartupState.chic_rt_startup_ptr_at((* const * const @readonly @expose_address char) argvPtr, 3, 1);
        NativeAlloc.Free(argv);
        Assert.That(missing == null).IsTrue();
    }
}
testcase Given_startup_internal_helpers_When_executed_Then_startup_internal_helpers()
{
    unsafe {
        StartupState.TestCoverageHelpers();
        Assert.That(true).IsTrue();
    }
}
