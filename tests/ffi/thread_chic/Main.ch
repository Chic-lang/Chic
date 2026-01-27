namespace Tests.FfiThreads;

import Std.Platform.Thread;
import Std.Sync;

public class Counter : ThreadStart
{
    private Arc<Mutex<int>> _shared;

    public init(Arc<Mutex<int>> shared)
    {
        _shared = shared;
    }

    public void Run()
    {
        var guard = _shared.Borrow().Lock();
        guard.ReplaceValue(1);
        guard.Release();
    }
}

public static int Main()
{
    var shared = new Arc<Mutex<int>>(new Mutex<int>(0));
    var thread = Thread.Spawn(ThreadStartFactory.From(new Counter(shared.Clone())));
    if (thread.Join() != ThreadStatus.Success)
    {
        return 1;
    }
    var guard = shared.Borrow().Lock();
    let joinedValue = Std.Numeric.NumericUnchecked.ToInt32(guard.Value);
    guard.Release();
    if (joinedValue != 1)
    {
        return 2;
    }

    var detachedShared = new Arc<Mutex<int>>(new Mutex<int>(0));
    var detached = Thread.Spawn(ThreadStartFactory.From(new Counter(detachedShared.Clone())));
    if (detached.Detach() != ThreadStatus.Success)
    {
        return 3;
    }
    var attempts = 0;
    var detachedValue = 0;
    while (attempts < 200)
    {
        var detachedGuard = detachedShared.Borrow().Lock();
        detachedValue = Std.Numeric.NumericUnchecked.ToInt32(detachedGuard.Value);
        detachedGuard.Release();
        if (detachedValue != 0)
        {
            break;
        }
        Thread.Sleep(1);
        attempts += 1;
    }
    if (detachedValue != 1)
    {
        return 4;
    }

    return 0;
}
