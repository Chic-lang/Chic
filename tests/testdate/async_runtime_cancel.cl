@allow(dead_code)
@allow(unused_param)
namespace AsyncRuntimeCancel;

import Std.Async;
import Std.Async.Runtime;

public async Task<int> PendingWork()
{
    await Ready();
    return 7;
}

public async Task<int> Ready()
{
    return 1;
}

public async Task<int> Main()
{
    var worker = PendingWork();
    var cancelled = Runtime.CancelInt(ref worker);
    RuntimeIntrinsics.chic_rt_async_cancel(RuntimeExports.TaskHeader(worker));
    Runtime.Spawn(worker);
    var result = await worker;
    if (!cancelled)
    {
        return -1;
    }
    if (result != 0)
    {
        return -2;
    }
    return 0;
}
