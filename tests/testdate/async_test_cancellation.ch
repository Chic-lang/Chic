@allow(dead_code)
@allow(unused_param)
namespace AsyncTestCancellation;

import Std.Async;
import Std.Async.Runtime;

async testcase RuntimeCancelCompletes()
{
    var worker = Busy();
    var cancelled = Runtime.CancelInt(ref worker);
    RuntimeIntrinsics.chic_rt_async_cancel(RuntimeExports.TaskHeader(worker));
    Runtime.Spawn(worker);
    var result = await worker;
    return cancelled && result == 0;
}

async testcase TokenCancellationCompletes()
{
    var source = CancellationTokenSource.Create();
    source.Cancel();
    var token = source.Token();
    await Tick();
    return source.IsCanceled || token.IsCancellationRequested();
}

public async Task<int> Busy()
{
    await Tick();
    return 5;
}

public async Task<int> Tick()
{
    return 1;
}

public int Main()
{
    return 0;
}
