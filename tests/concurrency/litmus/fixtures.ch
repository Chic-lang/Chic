namespace Tests.Concurrency.Litmus;

import Std.Platform.Thread;

public struct Pair
{
    public int First;
    public int Second;
}

public struct DoublePair
{
    public Pair Left;
    public Pair Right;
}

internal static class LitmusAssert
{
    public static void ThreadSucceeded(ThreadStatus status, string context)
    {
        if (status != ThreadStatus.Success)
        {
            throw new Std.InvalidOperationException(
                context + " failed: " + Std.Platform.Thread.ThreadStatusExtensions.ToString(status)
            );
        }
    }

    public static void Forbid(bool condition, string message)
    {
        if (condition)
        {
            throw new Std.InvalidOperationException(message);
        }
    }
}

internal sealed class StartGate
{
    private bool _armed;

    public init()
    {
        _armed = false;
    }

    public void Release()
    {
        _armed = true;
    }

    public void Wait()
    {
        while (!_armed)
        {
            Thread.SpinWait(32);
        }
    }
}

internal static class LitmusSpin
{
    public static void Delay()
    {
        Thread.SpinWait(256);
    }
}
