@allow(dead_code)
@allow(unused_param)
namespace AsyncTestcases;

import Std.Async;
import Std.Async.Runtime;

public async Task<int> Accumulate(int seed)
{
    return seed + 1;
}

async testcase AsyncPasses()
{
    var source = CancellationTokenSource.Create();
    var token = source.Token();
    await Accumulate(1);
    await Accumulate(2);
    token.IsCancellationRequested();
    return true;
}

async testcase AsyncAggregates()
{
    var token = CancellationTokenSource.Create().Token();
    await Accumulate(3);
    token.IsCancellationRequested();
    return true;
}

testcase SyncPasses()
{
    return true;
}

public int Main()
{
    return 0;
}
