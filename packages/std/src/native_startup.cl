@suppress_startup_descriptor namespace Std.Runtime.Startup;
import Std;
import Std.Core;
import Std.Numeric;
internal static class StartupConstants
{
    public const uint DescriptorVersion = 1u;
    public const uint EntryAsync = 0x0000_0001u;
    public const uint EntryRetI32 = 0x0000_0002u;
    public const uint EntryRetBool = 0x0000_0004u;
    public const uint EntryRetVoid = 0x0000_0008u;
    public const uint EntryParamArgs = 0x0000_0100u;
    public const uint EntryParamEnv = 0x0000_0200u;
    public const uint TestcaseAsync = 0x0000_0001u;
    public const int MissingEntryExit = 90;
    public const int InvalidDescriptorExit = 91;
    public const int AsyncTestcaseFailureExit = 92;
    public const int PanicExitCode = 101;
    public const int AbortExitCode = 134;
}
internal static class StartupFlags
{
    private static bool Has(uint flags, uint bit) {
        return(flags & bit) != 0u;
    }
    public static bool EntryIsAsync(uint flags) {
        return Has(flags, StartupConstants.EntryAsync);
    }
    public static bool EntryUsesArgs(uint flags) {
        return Has(flags, StartupConstants.EntryParamArgs);
    }
    public static bool EntryUsesEnv(uint flags) {
        return Has(flags, StartupConstants.EntryParamEnv);
    }
    public static bool TestIsAsync(uint flags) {
        return Has(flags, StartupConstants.TestcaseAsync);
    }
}
internal struct EntryDescriptorSnapshot
{
    internal * const @readonly @expose_address byte Function;
    internal uint Flags;
    internal uint Reserved;
}
internal struct TestSuiteDescriptorSnapshot
{
    internal * const @readonly @expose_address byte Cases;
    internal usize Length;
}
internal struct StartupDescriptorSnapshot
{
    internal uint Version;
    internal EntryDescriptorSnapshot Entry;
    internal TestSuiteDescriptorSnapshot Tests;
}
internal struct TestCaseDescriptorSnapshot
{
    internal * const @readonly @expose_address byte Function;
    internal * const @readonly @expose_address byte NamePointer;
    internal usize NameLength;
    internal uint Flags;
    internal uint Reserved;
}
internal static class RuntimeIntrinsics
{
    @extern("C") public static extern StartupDescriptorSnapshot chic_rt_startup_descriptor_snapshot();
    @extern("C") public static extern void chic_rt_startup_test_descriptor(* mut TestCaseDescriptorSnapshot dest, usize index);
    @extern("C") public static extern bool chic_rt_startup_has_run_tests_flag();
    @extern("C") public static extern int chic_rt_startup_call_entry(* const @readonly @expose_address byte function_ptr,
    uint flags, int argc, * mut * mut char argv, * mut * mut char envp);
    @extern("C") public static extern * mut @expose_address byte chic_rt_startup_call_entry_async(* const @readonly @expose_address byte function_ptr,
    uint flags, int argc, * mut * mut char argv, * mut * mut char envp);
    @extern("C") public static extern int chic_rt_startup_complete_entry_async(* mut @expose_address byte task_ptr,
    uint flags);
    @extern("C") public static extern int chic_rt_startup_call_testcase(* const @readonly @expose_address byte function_ptr);
    @extern("C") public static extern * mut @expose_address byte chic_rt_startup_call_testcase_async(* const @readonly @expose_address byte function_ptr);
    @extern("C") public static extern int chic_rt_startup_complete_testcase_async(* mut @expose_address byte task_ptr);
    @extern("C") public static extern * const @readonly @expose_address char chic_rt_startup_ptr_at(* const * const char list,
    int index, int limit);
    @extern("C") public static extern string chic_rt_startup_cstr_to_string(* const @readonly @expose_address char ptr);
    @extern("C") public static extern string chic_rt_startup_slice_to_string(* const @readonly @expose_address byte ptr,
    usize len);
    @extern("C") public static extern string chic_rt_startup_i32_to_string(int value);
    @extern("C") public static extern string chic_rt_startup_usize_to_string(usize value);
    @extern("C") public static extern void chic_rt_startup_exit(int code);
}
internal static class StartupRuntimeState
{
    @extern("C") public static extern void chic_rt_startup_store_state(int argc, * mut * mut char argv, * mut * mut char envp);
    @extern("C") public static extern int chic_rt_startup_raw_argc();
    @extern("C") public static extern * mut * mut char chic_rt_startup_raw_argv();
    @extern("C") public static extern * mut * mut char chic_rt_startup_raw_envp();
}
internal static class StartupState
{
    public static int Store(int argc, * mut * mut char argv, * mut * mut char envp) {
        StartupRuntimeState.chic_rt_startup_store_state(argc, argv, envp);
        return 0;
    }
    public static int ArgCount() {
        return StartupRuntimeState.chic_rt_startup_raw_argc();
    }
    public static * mut * mut char ArgVector() {
        return StartupRuntimeState.chic_rt_startup_raw_argv();
    }
    public static * mut * mut char EnvVector() {
        return StartupRuntimeState.chic_rt_startup_raw_envp();
    }
    public static * const @readonly @expose_address char ArgumentPointer(int index) {
        let argc = ArgCount();
        if (index <0 || index >= argc)
        {
            return Std.Numeric.Pointer.NullConst <char >();
        }
        let argv = ArgVector();
        if (argv == null)
        {
            return Std.Numeric.Pointer.NullConst <char >();
        }
        var ptr = Std.Numeric.Pointer.NullConst <char >();
        unsafe {
            ptr = RuntimeIntrinsics.chic_rt_startup_ptr_at((* const * const char) argv, index, argc);
        }
        return ptr;
    }
    public static * const @readonly @expose_address char EnvironmentPointer(int index) {
        if (index <0)
        {
            return Std.Numeric.Pointer.NullConst <char >();
        }
        let envp = EnvVector();
        if (envp == null)
        {
            return Std.Numeric.Pointer.NullConst <char >();
        }
        var ptr = Std.Numeric.Pointer.NullConst <char >();
        unsafe {
            ptr = RuntimeIntrinsics.chic_rt_startup_ptr_at((* const * const char) envp, index, - 1);
        }
        return ptr;
    }
}
internal static class StartupStrings
{
    public static bool ArgumentEquals(int index, string literal) {
        let argc = StartupState.ArgCount();
        if (index <0 || index >= argc)
        {
            return false;
        }
        let pointer = StartupState.ArgumentPointer(index);
        if (pointer == null)
        {
            return false;
        }
        let argument = RuntimeIntrinsics.chic_rt_startup_cstr_to_string(pointer);
        return argument == literal;
    }
}
internal static class StartupDiagnostics
{
    private const string TestcasePrefix = "CHIC_TESTCASE\t";
    private static void EmitTestcase(string status, usize index, string name, string ?message) {
        let indexText = RuntimeIntrinsics.chic_rt_startup_usize_to_string(index);
        var line = TestcasePrefix + indexText + "\t" + status + "\t" + name;
        if (message != null && message.Length != 0)
        {
            line = line + "\t" + message;
        }
        Console.WriteLine(line);
    }
    public static int ReportMissingEntry() {
        Console.Error.WriteLine("startup: missing entry point");
        return StartupConstants.MissingEntryExit;
    }
    public static int ReportInvalidDescriptor() {
        Console.Error.WriteLine("startup: invalid descriptor");
        return StartupConstants.InvalidDescriptorExit;
    }
    public static int ReportAsyncEntryLaunchFailure() {
        Console.Error.WriteLine("startup: async entry failed to launch");
        return StartupConstants.AsyncTestcaseFailureExit;
    }
    public static int ReportAsyncTestcaseLaunchFailure(usize index) {
        EmitTestcase("FAIL", index, TestcaseName(index), "async testcase failed to launch");
        return StartupConstants.AsyncTestcaseFailureExit;
    }
    public static int ReportUnnamedTestFailure(usize index, int code) {
        let codeText = RuntimeIntrinsics.chic_rt_startup_i32_to_string(code);
        EmitTestcase("FAIL", index, TestcaseName(index), "testcase failed with code " + codeText);
        return 1;
    }
    public static int ReportNoTestcases() {
        Console.Error.WriteLine("startup: no testcases discovered");
        return 0;
    }
    public static int ReportPanic(int code) {
        let codeText = RuntimeIntrinsics.chic_rt_startup_i32_to_string(code);
        Console.Error.WriteLine("panic: chic panic code " + codeText);
        return StartupConstants.PanicExitCode;
    }
    public static int ReportAbort(int code) {
        let codeText = RuntimeIntrinsics.chic_rt_startup_i32_to_string(code);
        Console.Error.WriteLine("abort: chic abort code " + codeText);
        return StartupConstants.AbortExitCode;
    }
    public static void ReportTestcasePass(usize index, string name) {
        EmitTestcase("PASS", index, name, null);
    }
    public static void ReportTestcaseSkip(usize index, string name, string message) {
        EmitTestcase("SKIP", index, name, message);
    }
    public static void ReportTestcaseFail(usize index, string name, string message) {
        EmitTestcase("FAIL", index, name, message);
    }
    private static string TestcaseName(usize index) {
        let indexText = RuntimeIntrinsics.chic_rt_startup_usize_to_string(index);
        return "testcase#" + indexText;
    }
}
internal static class StartupEnvironment
{
    public static string ReadValue(string key) {
        if (key == null || key.Length == 0)
        {
            return "";
        }
        let envp = StartupState.EnvVector();
        if (envp == null)
        {
            return "";
        }
        let keyLen = key.Length;
        var idx = 0;
        while (true)
        {
            var ptr = Std.Numeric.Pointer.NullConst <char >();
            unsafe {
                ptr = RuntimeIntrinsics.chic_rt_startup_ptr_at((* const * const char) envp, idx, - 1);
            }
            if (ptr == null)
            {
                return "";
            }
            let entry = RuntimeIntrinsics.chic_rt_startup_cstr_to_string(ptr);
            if (entry.Length >keyLen && entry.StartsWith (key) && entry[keyLen] == '=')
            {
                return entry.Substring(keyLen + 1);
            }
            idx += 1;
        }
        return "";
    }
    public static bool IsTruthy(string key) {
        let value = ReadValue(key);
        if (value.Length == 0)
        {
            return false;
        }
        if (value == "0" || value == "false" || value == "False" || value == "FALSE")
        {
            return false;
        }
        return true;
    }
}
internal static class StartupTestSelection
{
    private const string SelectionKey = "CHIC_TEST_INDEXES";
    private const string FailFastKey = "CHIC_TEST_FAIL_FAST";
    public static string SelectionValue() {
        return StartupEnvironment.ReadValue(SelectionKey);
    }
    public static bool FailFastEnabled() {
        return StartupEnvironment.IsTruthy(FailFastKey);
    }
    public static bool AllowsIndex(usize index, string selection) {
        if (selection.Length == 0)
        {
            return true;
        }
        let length = selection.Length;
        var cursor = 0;
        while (cursor <length)
        {
            let ch = selection[cursor];
            if (ch <'0' || ch >'9')
            {
                cursor += 1;
                continue;
            }
            var value = 0usize;
            while (cursor <length)
            {
                let digit = selection[cursor];
                if (digit <'0' || digit >'9')
                {
                    break;
                }
                value = (value * 10usize) + (usize)(NumericUnchecked.ToInt32(digit) - NumericUnchecked.ToInt32('0'));
                cursor += 1;
            }
            if (value == index)
            {
                return true;
            }
        }
        return false;
    }
}
public static class NativeStartup
{
    public static int Main(int argc, * mut * mut char argv, * mut * mut char envp) {
        StartupState.Store(argc, argv, envp);
        let descriptor = RuntimeIntrinsics.chic_rt_startup_descriptor_snapshot();
        if (descriptor.Version != StartupConstants.DescriptorVersion)
        {
            StartupDiagnostics.ReportInvalidDescriptor();
            return StartupConstants.InvalidDescriptorExit;
        }
        if (ShouldRunTests ())
        {
            return RunTests(descriptor.Tests);
        }
        return RunEntry(descriptor.Entry);
    }
    private static bool ShouldRunTests() {
        return RuntimeIntrinsics.chic_rt_startup_has_run_tests_flag();
    }
    private static int RunEntry(EntryDescriptorSnapshot entry) {
        if (entry.Function == null)
        {
            StartupDiagnostics.ReportMissingEntry();
            return StartupConstants.MissingEntryExit;
        }
        let wantsArgs = StartupFlags.EntryUsesArgs(entry.Flags);
        let wantsEnv = StartupFlags.EntryUsesEnv(entry.Flags);
        let argc = wantsArgs ?StartupState.ArgCount() : 0;
        let argv = StartupState.ArgVector();
        let envp = StartupState.EnvVector();
        var result = 0;
        if (StartupFlags.EntryIsAsync (entry.Flags))
        {
            result = RunAsyncEntry(entry, argc, argv, envp);
        }
        else
        {
            result = RuntimeIntrinsics.chic_rt_startup_call_entry(entry.Function, entry.Flags, argc, argv, envp);
        }
        return result;
    }
    private static int RunAsyncEntry(EntryDescriptorSnapshot entry, int argc, * mut * mut char argv, * mut * mut char envp) {
        let taskPtr = RuntimeIntrinsics.chic_rt_startup_call_entry_async(entry.Function, entry.Flags, argc, argv, envp);
        if (taskPtr == null)
        {
            StartupDiagnostics.ReportAsyncEntryLaunchFailure();
            return StartupConstants.AsyncTestcaseFailureExit;
        }
        return RuntimeIntrinsics.chic_rt_startup_complete_entry_async(taskPtr, entry.Flags);
    }
    private static int RunTests(TestSuiteDescriptorSnapshot suite) {
        if (suite.Length == 0)
        {
            StartupDiagnostics.ReportNoTestcases();
            return 0;
        }
        let selection = StartupTestSelection.SelectionValue();
        let failFast = StartupTestSelection.FailFastEnabled();
        var failures = 0usize;
        var index = 0usize;
        var test = CoreIntrinsics.DefaultValue <TestCaseDescriptorSnapshot >();
        while (index <suite.Length)
        {
            if (! StartupTestSelection.AllowsIndex (index, selection))
            {
                index += 1;
                continue;
            }
            unsafe {
                RuntimeIntrinsics.chic_rt_startup_test_descriptor(& mut test, index);
            }
            let name = ResolveTestName(test, index);
            if (test.Function == null)
            {
                StartupDiagnostics.ReportTestcaseFail(index, name, "missing testcase entry point");
                failures += 1;
                if (failFast)
                {
                    break;
                }
                index += 1;
                continue;
            }
            let isAsync = StartupFlags.TestIsAsync(test.Flags);
            var result = 0;
            if (isAsync)
            {
                result = RunAsyncTestcase(test, index);
                if (result == StartupConstants.AsyncTestcaseFailureExit)
                {
                    failures += 1;
                    StartupDiagnostics.ReportTestcaseFail(index, name, "async testcase failed");
                    if (failFast)
                    {
                        break;
                    }
                    index += 1;
                    continue;
                }
            }
            else
            {
                result = RuntimeIntrinsics.chic_rt_startup_call_testcase(test.Function);
            }
            if (result != 0)
            {
                let codeText = RuntimeIntrinsics.chic_rt_startup_i32_to_string(result);
                StartupDiagnostics.ReportTestcaseFail(index, name, "testcase returned " + codeText);
                failures += 1;
                if (failFast)
                {
                    break;
                }
            }
            else
            {
                StartupDiagnostics.ReportTestcasePass(index, name);
            }
            index += 1;
        }
        if (failures == 0)
        {
            return 0;
        }
        return 1;
    }
    private static int RunAsyncTestcase(TestCaseDescriptorSnapshot test, usize index) {
        let taskPtr = RuntimeIntrinsics.chic_rt_startup_call_testcase_async(test.Function);
        return RuntimeIntrinsics.chic_rt_startup_complete_testcase_async(taskPtr);
    }
    private static string ResolveTestName(TestCaseDescriptorSnapshot test, usize index) {
        if (test.NamePointer == null || test.NameLength == 0usize)
        {
            let indexText = RuntimeIntrinsics.chic_rt_startup_usize_to_string(index);
            return "testcase#" + indexText;
        }
        return RuntimeIntrinsics.chic_rt_startup_slice_to_string(test.NamePointer, test.NameLength);
    }
    public static int GetArgc() {
        return StartupState.ArgCount();
    }
    public static * const @readonly @expose_address char GetArgumentPointer(int index) {
        return StartupState.ArgumentPointer(index);
    }
    public static * const @readonly @expose_address char GetEnvironmentPointer(int index) {
        return StartupState.EnvironmentPointer(index);
    }
    public static int Panic(int code) {
        return PanicInternal(code);
    }
    public static int Abort(int code) {
        return AbortInternal(code);
    }
    private static int PanicInternal(int code) {
        StartupDiagnostics.ReportPanic(code);
        return ExitProcess(StartupConstants.PanicExitCode);
    }
    private static int AbortInternal(int code) {
        StartupDiagnostics.ReportAbort(code);
        return ExitProcess(StartupConstants.AbortExitCode);
    }
    private static int ExitProcess(int exitCode) {
        RuntimeIntrinsics.chic_rt_startup_exit(exitCode);
        return exitCode;
    }
}
