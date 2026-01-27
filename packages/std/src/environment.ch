import Std.Collections;
import Std.Platform;
namespace Std
{
    public static class Environment
    {
        public static string OsDescription() => EnvironmentInfo.OsDescription();
        public static string Architecture() => EnvironmentInfo.Architecture();
        public static int ProcessId() => EnvironmentInfo.ProcessId();
        public static string WorkingDirectory() => EnvironmentInfo.WorkingDirectory();
        public static string NewLine() => EnvironmentInfo.NewLine();
        public static ulong TickCountMilliseconds() => EnvironmentInfo.TickCountMilliseconds();
        public static ulong UptimeMilliseconds() => EnvironmentInfo.UptimeMilliseconds();
        public static string ?GetEnvironmentVariable(string name) => EnvironmentVariables.Get(name);
        public static bool SetEnvironmentVariable(string name, string value) => EnvironmentVariables.Set(name, value);
        public static bool RemoveEnvironmentVariable(string name) => EnvironmentVariables.Remove(name);
        public static VecPtr EnumerateEnvironment() => EnvironmentVariables.Enumerate();
        public static VecPtr CommandLine() => ProcessInfo.CommandLine();
        public static void Exit(int code) => ProcessInfo.Exit(code);
    }
}
