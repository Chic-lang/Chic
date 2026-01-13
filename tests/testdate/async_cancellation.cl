namespace AsyncCancellation;

import Std.Async;
import Std.Async.Runtime;

public async Task DelayThenCancel(CancellationTokenSource source)
{
    source.Cancel();
    if (source.IsCanceled)
    {
        return;
    }
}

public async Task<int> Main()
{
    var source = CancellationTokenSource.Create();
    var token = source.Token();
    if (token.IsCancellationRequested())
    {
        return -2;
    }
    await DelayThenCancel(source);
    if (token.IsCancellationRequested())
    {
        return 0;
    }
    return 3;
}
