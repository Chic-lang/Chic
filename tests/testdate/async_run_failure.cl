@allow(dead_code)
@allow(unused_param)
namespace AsyncRunFailure;

import Std.Async;

public async Task<int> Fail()
{
    return 3;
}

public async Task<int> Main()
{
    var value = await Fail();
    return value;
}
