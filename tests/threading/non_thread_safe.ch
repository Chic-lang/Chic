namespace Samples.Threads;

import Std.Platform.Thread;

@not_thread_safe
public class NotSafeWorker : ThreadStart
{
    public void Run()
    {
        // Intentionally empty; the attribute forbids sending this type across threads.
    }
}

public static class Program
{
    public static int Main()
    {
        var worker = ThreadStartFactory.From(new NotSafeWorker());
        Thread.Spawn(worker);
        return 0;
    }
}
