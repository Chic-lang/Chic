namespace Samples.Threads;

import Std.Sync;
import Std.Platform.Thread;

internal static class ThreadHelpers
{
    public static Mutex<int> FunctionMutex;

    public static void IncrementSharedMutex()
    {
        var guard = FunctionMutex.Lock();
        guard.Value += 1;
        guard.Release();
    }
}

public class CounterThread : ThreadStart
{
    private Mutex<int> _mutex;

    public init(Mutex<int> mutex)
    {
        _mutex = mutex;
    }

    public void Run()
    {
        var guard = _mutex.Lock();
        guard.Value += 1;
        guard.Release();
    }
}

public struct CondvarPayload
{
    public bool Ready;
    public int Result;
}

public class CondvarThread : ThreadStart
{
    private Mutex<CondvarPayload> _mutex;
    private Condvar _cond;

    public init(Mutex<CondvarPayload> mutex, Condvar cond)
    {
        _mutex = mutex;
        _cond = cond;
    }

    public void Run()
    {
        var guard = _mutex.Lock();
        guard.Value.Result = 2;
        guard.Value.Ready = true;
        guard.Release();
        _cond.NotifyOne();
    }
}

public static class Program
{
    public static int Main()
    {
        ThreadsIncrement();
        BuilderSpawn();
        BuilderTruncates();
        FunctionAdapterSpawn();
        CondvarCoordinates();
        SleepAndYield();
        return 0;
    }

    private static void ThreadsIncrement()
    {
        var mutex = new Mutex<int>(0);
        var runner = ThreadStartFactory.From(new CounterThread(mutex));
        var thread = Thread.Spawn(runner);
        var status = thread.Join();
        if (status != ThreadStatus.Success)
        {
            throw new Std.InvalidOperationException("thread join failed");
        }
        var guard = mutex.Lock();
        if (guard.Value != 1)
        {
            throw new Std.InvalidOperationException("counter mismatch");
        }
        guard.Release();
    }

    private static void BuilderSpawn()
    {
        var mutex = new Mutex<int>(0);
        var runner = ThreadStartFactory.From(new CounterThread(mutex));
        var builder = new ThreadBuilder().WithName("worker");
        var thread = builder.Spawn(runner);
        var status = thread.Join();
        if (status != ThreadStatus.Success)
        {
            throw new Std.InvalidOperationException("builder thread join failed");
        }
        if (thread.Name != "worker")
        {
            throw new Std.InvalidOperationException("thread name was not propagated");
        }
    }

    private static void BuilderTruncates()
    {
        var runner = ThreadStartFactory.Function(fn () {
            Thread.Sleep(1);
        });
        // 16 characters; limit is 15 bytes so this should truncate the tail.
        let longName = "abcdefghijklmnop";
        var thread = new ThreadBuilder().WithName(longName).Spawn(runner);
        let expected = "abcdefghijklmno";
        if (thread.Name != expected)
        {
            throw new Std.InvalidOperationException("thread name was not truncated deterministically");
        }
        let _ = thread.Join();
    }

    private static void FunctionAdapterSpawn()
    {
        ThreadHelpers.FunctionMutex = new Mutex<int>(0);
        var thread = Thread.Spawn(ThreadStartFactory.Function(ThreadHelpers.IncrementSharedMutex));
        var status = thread.Join();
        if (status != ThreadStatus.Success)
        {
            throw new Std.InvalidOperationException("function adapter thread join failed");
        }
        var guard = ThreadHelpers.FunctionMutex.Lock();
        if (guard.Value != 1)
        {
            throw new Std.InvalidOperationException("function adapter mismatch");
        }
        guard.Release();
    }

    private static void CondvarCoordinates()
    {
        var mutex = new Mutex<CondvarPayload>(new CondvarPayload { Ready = false, Result = 0 });
        var cond = new Condvar();
        var thread = Thread.Spawn(ThreadStartFactory.From(new CondvarThread(mutex, cond)));
        var guard = mutex.Lock();
        while (!guard.Value.Ready)
        {
            guard = cond.Wait(mutex, guard);
        }
        if (guard.Value.Result != 2)
        {
            throw new Std.InvalidOperationException("condvar payload mismatch");
        }
        guard.Release();
        var status = thread.Join();
        if (status != ThreadStatus.Success)
        {
            throw new Std.InvalidOperationException("condvar thread join failed");
        }
    }

    private static void SleepAndYield()
    {
        Thread.Sleep(1);
        Thread.Yield();
        Thread.SpinWait(4);
    }
}
