@allow(dead_code)
@allow(unused_param)
namespace AsyncTestTimeout;

import Std.Async;

async testcase HangsForever()
{
    var value = 0;
    while (true)
    {
        value += await Step();
    }
    return value == -1;
}

public async Task<int> Step()
{
    return 1;
}

public int Main()
{
    return 0;
}
