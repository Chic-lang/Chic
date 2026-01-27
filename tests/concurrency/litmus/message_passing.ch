namespace Tests.Concurrency.Litmus;

import Std.Platform.Thread;

internal static class MessagePassingScenario
{
    private static int _flag;
    private static int _data;
    private static int _observed;
    private static int _payload;
    private static StartGate _gate;

    public static int RunIteration(int payload)
    {
        _flag = 0;
        _data = 0;
        _observed = -1;
        _payload = payload;
        _gate = new StartGate();

        var publisher = Thread.Spawn(ThreadStartFactory.Function(Publish));
        var consumer = Thread.Spawn(ThreadStartFactory.Function(Consume));

        _gate.Release();

        LitmusAssert.ThreadSucceeded(publisher.Join(), "message-passing publisher");
        LitmusAssert.ThreadSucceeded(consumer.Join(), "message-passing consumer");

        return _observed;
    }

    private static void Publish()
    {
        _gate.Wait();
        var value = _payload;
        _data = value;
        _flag = 1;
    }

    private static void Consume()
    {
        _gate.Wait();
        while (_flag == 0)
        {
            Thread.Yield();
        }
        var observed = _data;
        _observed = observed;
    }
}

testcase MessagePassingTransfersPublishedValue()
{
    for (var iteration = 0; iteration < 512; iteration += 1)
    {
        var payload = 10_000 + iteration;
        var observed = MessagePassingScenario.RunIteration(payload);
        LitmusAssert.Forbid(
            observed != payload,
            "message passing observed stale payload after Acquire flag"
        );
    }
}
