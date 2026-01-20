namespace Std.Runtime.Bootstrap;
import Std.Runtime.Startup;
public static class NativeMain
{
    private const uint EntryFlagAsync = 0x00000001u;
    private const int MissingEntryExit = 90;
    private const int UnsupportedAsyncExit = 93;
    @extern("C") private static extern int chic_rt_test_executor_run_all();
    @extern("C") @weak @export("main") public static int Main(int argc, * mut * mut char argv, * mut * mut char envp) {
        StartupRuntimeState.chic_rt_startup_store_state(argc, argv, envp);
        if (RuntimeIntrinsics.chic_rt_startup_has_run_tests_flag ())
        {
            return chic_rt_test_executor_run_all();
        }
        let descriptor = RuntimeIntrinsics.chic_rt_startup_descriptor_snapshot();
        if ( (descriptor.Entry.Flags & EntryFlagAsync) != 0)
        {
            return UnsupportedAsyncExit;
        }
        return RuntimeIntrinsics.chic_rt_startup_call_entry(descriptor.Entry.Function, descriptor.Entry.Flags, argc,
        argv, envp);
    }
}
