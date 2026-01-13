namespace Tests.Concurrency.Litmus;

import Std.Platform.Thread;
import Std.Core;

internal static class LoadBufferingScenario
{
    private static int _x;
    private static int _y;
    private static int _left;
    private static int _right;
    private static bool _actorZeroFirst = true;
    private static StartGate _gate;

    public static Pair RunIteration(bool preferZeroFirst)
    {
        _x = 0;
        _y = 0;
        _left = -1;
        _right = -1;
        _gate = new StartGate();
        _actorZeroFirst = preferZeroFirst;

        var actorZero = Thread.Spawn(ThreadStartFactory.Function(ActorZero));
        var actorOne = Thread.Spawn(ThreadStartFactory.Function(ActorOne));

        _gate.Release();

        LitmusAssert.ThreadSucceeded(actorZero.Join(), "load-buffering actor zero");
        LitmusAssert.ThreadSucceeded(actorOne.Join(), "load-buffering actor one");

        var outcome = CoreIntrinsics.DefaultValue<Pair>();
        outcome.First = _left;
        outcome.Second = _right;
        return outcome;
    }

    private static void ActorZero()
    {
        _gate.Wait();
        if (!_actorZeroFirst)
        {
            LitmusSpin.Delay();
        }
        var observed = _y;
        _x = 1;
        _left = observed;
    }

    private static void ActorOne()
    {
        _gate.Wait();
        if (_actorZeroFirst)
        {
            LitmusSpin.Delay();
        }
        var observed = _x;
        _y = 1;
        _right = observed;
    }
}

testcase LoadBufferingRejectsOneOne()
{
    var preferZero = true;
    for (var iteration = 0; iteration < 1024; iteration += 1)
    {
        var outcome = LoadBufferingScenario.RunIteration(preferZero);
        preferZero = !preferZero;
        LitmusAssert.Forbid(
            outcome.First == 1 && outcome.Second == 1,
            "forbidden load-buffering pair (1, 1) observed under Acquire/Release ordering"
        );
    }
}
