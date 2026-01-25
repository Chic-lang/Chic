@allow(dead_code)
@allow(unused_param)
namespace AsyncTimeout;

import Std.Async;

public async Task<int> Spin()
{
    var accumulator = 0;
    while (accumulator >= 0)
    {
        accumulator += await Tick();
    }
    return accumulator;
}

public async Task<int> Tick()
{
    return 1;
}

public async Task<int> Main()
{
    return await Spin();
}
