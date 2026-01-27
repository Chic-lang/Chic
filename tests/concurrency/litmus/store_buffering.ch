namespace Tests.Concurrency.Litmus;

import Std.Platform.Thread;
import Std.Core;

internal sealed class StoreBufferingState
{
    public int X;
    public int Y;
    public int Left;
    public int Right;
    public bool ActorZeroFirst;
    public StartGate Gate = new StartGate();

    public void Reset()
    {
        X = 0;
        Y = 0;
        Left = -1;
        Right = -1;
        Gate = new StartGate();
    }
}

internal sealed class StoreBufferingActorZero : ThreadStart
{
    private StoreBufferingState _state;

    public init(StoreBufferingState state)
    {
        _state = state;
    }

    public void Run()
    {
        _state.Gate.Wait();
        if (!_state.ActorZeroFirst)
        {
            LitmusSpin.Delay();
        }
        _state.X = 1;
        _state.Left = _state.Y;
    }
}

internal sealed class StoreBufferingActorOne : ThreadStart
{
    private StoreBufferingState _state;

    public init(StoreBufferingState state)
    {
        _state = state;
    }

    public void Run()
    {
        _state.Gate.Wait();
        if (_state.ActorZeroFirst)
        {
            LitmusSpin.Delay();
        }
        _state.Y = 1;
        _state.Right = _state.X;
    }
}

internal static class StoreBufferingScenario
{
    public static Pair RunIteration(bool preferZeroFirst)
    {
        var state = new StoreBufferingState();
        state.ActorZeroFirst = preferZeroFirst;
        state.Reset();

        var actorZero = Thread.Spawn(ThreadStartFactory.From(new StoreBufferingActorZero(state)));
        var actorOne = Thread.Spawn(ThreadStartFactory.From(new StoreBufferingActorOne(state)));

        state.Gate.Release();

        LitmusAssert.ThreadSucceeded(actorZero.Join(), "store-buffering actor zero");
        LitmusAssert.ThreadSucceeded(actorOne.Join(), "store-buffering actor one");

        var outcome = CoreIntrinsics.DefaultValue<Pair>();
        outcome.First = state.Left;
        outcome.Second = state.Right;
        return outcome;
    }
}

testcase StoreBufferingRejectsZeroZero()
{
    var preferZero = true;
    for (var iteration = 0; iteration < 1024; iteration += 1)
    {
        var outcome = StoreBufferingScenario.RunIteration(preferZero);
        preferZero = !preferZero;
        LitmusAssert.Forbid(
            outcome.First == 0 && outcome.Second == 0,
            "forbidden store-buffering pair (0, 0) observed under Acquire/Release ordering"
        );
    }
}
