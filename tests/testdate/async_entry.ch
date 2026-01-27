@allow(dead_code)
@allow(unused_param)
namespace AsyncEntry;

import Std.Async;
import Std.Async.Runtime;

public async Task<int> Compute(int value)
{
    // Simple async helper that remains awaitable even when the runtime completes immediately.
    return value + 2;
}

public async Task<int> Chain()
{
    var source = CancellationTokenSource.Create();
    var token = source.Token();
    if (token.IsCancellationRequested())
    {
        return -1;
    }
    await Compute(2);
    await Compute(4);
    return 5;
}

public async Task<int> Main()
{
    var value = await Chain();
    return value + 2;
}
