// Minimal startup surface so async CLI tests can run without the full standard library.
namespace Std.Runtime.Startup;

import Std.Async;
import Std.Async.Runtime;
import Std.Runtime;

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
    private static bool Has(uint flags, uint bit)
    {
        return (flags & bit) != 0u;
    }

    public static bool EntryIsAsync(uint flags)
    {
        return Has(flags, StartupConstants.EntryAsync);
    }

    public static bool EntryUsesArgs(uint flags)
    {
        return Has(flags, StartupConstants.EntryParamArgs);
    }

    public static bool EntryUsesEnv(uint flags)
    {
        return Has(flags, StartupConstants.EntryParamEnv);
    }

    public static bool TestIsAsync(uint flags)
    {
        return Has(flags, StartupConstants.TestcaseAsync);
    }
}

@repr(c) public struct EntryDescriptorSnapshot
{
    public *const @readonly @expose_address byte Function;
    public uint Flags;
    public uint Reserved;
}

@repr(c) public struct TestSuiteDescriptorSnapshot
{
    public *const @readonly @expose_address byte Cases;
    public usize Length;
}

@repr(c) public struct StartupDescriptorSnapshot
{
    public uint Version;
    public EntryDescriptorSnapshot Entry;
    public TestSuiteDescriptorSnapshot Tests;
}

@repr(c) public struct TestCaseDescriptorSnapshot
{
    public *const @readonly @expose_address byte Function;
    public *const @readonly @expose_address byte NamePointer;
    public usize NameLength;
    public uint Flags;
    public uint Reserved;
}

@repr(c) public struct ValueMutPtr
{
    public *mut @expose_address byte Pointer;
    public usize Size;
    public usize Alignment;
}

internal static class StartupRuntimeState
{
    @extern("C")
    public static extern StartupDescriptorSnapshot chic_rt_startup_descriptor_snapshot();

    @extern("C")
    public static extern void chic_rt_startup_test_descriptor(
        *mut TestCaseDescriptorSnapshot dest,
        usize index
    );

    @extern("C")
    public static extern bool chic_rt_startup_has_run_tests_flag();

    @extern("C")
    public static extern int chic_rt_test_executor_run_all();

    @extern("C")
    public static extern int chic_rt_startup_call_entry(
        *const @readonly @expose_address byte function_ptr,
        uint flags,
        int argc,
        *mut *mut char argv,
        *mut *mut char envp
    );

    @extern("C")
    public static extern *mut @expose_address byte chic_rt_startup_call_entry_async(
        *const @readonly @expose_address byte function_ptr,
        uint flags,
        int argc,
        *mut *mut char argv,
        *mut *mut char envp
    );

    @extern("C")
    public static extern int chic_rt_startup_complete_entry_async(
        *mut @expose_address byte task_ptr,
        uint flags
    );

    @extern("C")
    public static extern int chic_rt_startup_call_testcase(
        *const @readonly @expose_address byte function_ptr
    );

    @extern("C")
    public static extern *mut @expose_address byte chic_rt_startup_call_testcase_async(
        *const @readonly @expose_address byte function_ptr
    );

    @extern("C")
    public static extern int chic_rt_startup_complete_testcase_async(
        *mut @expose_address byte task_ptr
    );

    @extern("C")
    public static extern isize chic_rt_startup_ptr_at(isize list, int index, int limit);

    @extern("C")
    public static extern void chic_rt_startup_exit(int code);

    @extern("C")
    public static extern void chic_rt_startup_store_state(
        int argc,
        *mut *mut char argv,
        *mut *mut char envp
    );

    @extern("C")
    public static extern int chic_rt_startup_raw_argc();

    @extern("C")
    public static extern *mut *mut char chic_rt_startup_raw_argv();

    @extern("C")
    public static extern *mut *mut char chic_rt_startup_raw_envp();
}

internal static class StartupState
{
    public static int Store(int argc, *mut *mut char argv, *mut *mut char envp)
    {
        StartupRuntimeState.chic_rt_startup_store_state(argc, argv, envp);
        return 0;
    }

    public static int ArgCount()
    {
        return StartupRuntimeState.chic_rt_startup_raw_argc();
    }

    public static *mut *mut char ArgVector()
    {
        return StartupRuntimeState.chic_rt_startup_raw_argv();
    }

    public static *mut *mut char EnvVector()
    {
        return StartupRuntimeState.chic_rt_startup_raw_envp();
    }
}

internal static class StartupDiagnostics
{
    public static int ReportMissingEntry() { return 0; }
    public static int ReportInvalidDescriptor() { return 0; }
    public static int ReportAsyncEntryLaunchFailure() { return 0; }
    public static int ReportAsyncTestcaseLaunchFailure(usize _index) { return 0; }
    public static int ReportUnnamedTestFailure(usize _index, int _code) { return 0; }
    public static int ReportNoTestcases() { return 0; }
    public static int ReportPanic(int _code) { return 0; }
    public static int ReportAbort(int _code) { return 0; }
}

public static class NativeStartup
{
    // Native host entry point shim; not considered the language-level Main.
    @export("main")
    public static int BootstrapMain(int argc, *mut *mut char argv, *mut *mut char envp)
    {
        StartupState.Store(argc, argv, envp);
        return ChicMain();
    }

    // Wasm executor looks for a zero-argument `chic_main` export.
    @export("chic_main")
    public static int ChicMain()
    {
        var argc = StartupState.ArgCount();
        return MainInner(argc);
    }

    private static int MainInner(int argc)
    {
        let descriptor = StartupRuntimeState.chic_rt_startup_descriptor_snapshot();
        if (descriptor.Version != StartupConstants.DescriptorVersion)
        {
            StartupDiagnostics.ReportInvalidDescriptor();
            return StartupConstants.InvalidDescriptorExit;
        }

        if (StartupRuntimeState.chic_rt_startup_has_run_tests_flag())
        {
            return StartupRuntimeState.chic_rt_test_executor_run_all();
        }

        if (descriptor.Entry.Function == null)
        {
            StartupDiagnostics.ReportMissingEntry();
            return StartupConstants.MissingEntryExit;
        }

        return RunEntry(descriptor.Entry);
    }

    private static int RunEntry(EntryDescriptorSnapshot entry)
    {
        let wantsArgs = StartupFlags.EntryUsesArgs(entry.Flags);
        var argc = StartupState.ArgCount();
        var argv = StartupState.ArgVector();
        var envp = StartupState.EnvVector();
        if (!wantsArgs)
        {
            argc = 0;
        }

        if (StartupFlags.EntryIsAsync(entry.Flags))
        {
            let taskPtr = StartupRuntimeState.chic_rt_startup_call_entry_async(
                entry.Function,
                entry.Flags,
                argc,
                argv,
                envp
            );
            return StartupRuntimeState.chic_rt_startup_complete_entry_async(
                taskPtr,
                entry.Flags
            );
        }

        return StartupRuntimeState.chic_rt_startup_call_entry(
            entry.Function,
            entry.Flags,
            argc,
            argv,
            envp
        );
    }

    private static int RunTests(TestSuiteDescriptorSnapshot suite)
    {
        let _ = suite;
        return StartupRuntimeState.chic_rt_test_executor_run_all();
    }

    @export("chic_rt_startup_argc")
    public static int GetArgc()
    {
        return StartupState.ArgCount();
    }

    @export("chic_rt_startup_argv")
    public unsafe static *const @readonly @expose_address char GetArgumentPointer(int index)
    {
        var argv = StartupState.ArgVector();
        if (argv == null)
        {
            return (*const @readonly @expose_address char)0;
        }
        return StartupRuntimeState.chic_rt_startup_ptr_at(
            (*const *const char)argv,
            index,
            StartupState.ArgCount()
        );
    }

    @export("chic_rt_startup_env")
    public unsafe static *const @readonly @expose_address char GetEnvironmentPointer(int index)
    {
        var envp = StartupState.EnvVector();
        if (envp == null)
        {
            return (*const @readonly @expose_address char)0;
        }
        return StartupRuntimeState.chic_rt_startup_ptr_at(
            (*const *const char)envp,
            index,
            -1
        );
    }

    @weak @export("chic_rt_panic")
    public static int Panic(int code)
    {
        StartupDiagnostics.ReportPanic(code);
        StartupRuntimeState.chic_rt_startup_exit(StartupConstants.PanicExitCode);
        return StartupConstants.PanicExitCode;
    }

    @weak @export("chic_rt_abort")
    public static int Abort(int code)
    {
        StartupDiagnostics.ReportAbort(code);
        StartupRuntimeState.chic_rt_startup_exit(StartupConstants.AbortExitCode);
        return StartupConstants.AbortExitCode;
    }

    // Stubbed exports to satisfy the native runtime while the shim is being removed.
    @weak @export("chic_thread_invoke")
    public static void ThreadInvoke(ValueMutPtr context)
    {
        let _ = context;
    }

    @weak @export("chic_thread_drop")
    public static void ThreadDrop(ValueMutPtr context)
    {
        let _ = context;
    }
}
