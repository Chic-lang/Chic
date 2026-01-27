namespace Tests.Concurrency.Litmus;

import Std.Platform.Thread;
import Std.Core;

internal static class IriwScenario
{
    private static int _x;
    private static int _y;
    private static bool _writerXFirst = true;
    private static StartGate _gate;
    private static int _readerXYFirst;
    private static int _readerXYSecond;
    private static int _readerYXFirst;
    private static int _readerYXSecond;

    public static DoublePair RunIteration(bool writeXFirst)
    {
        _x = 0;
        _y = 0;
        _readerXYFirst = -1;
        _readerXYSecond = -1;
        _readerYXFirst = -1;
        _readerYXSecond = -1;
        _writerXFirst = writeXFirst;
        _gate = new StartGate();

        var writerX = Thread.Spawn(ThreadStartFactory.Function(WriterX));
        var writerY = Thread.Spawn(ThreadStartFactory.Function(WriterY));
        var readerXY = Thread.Spawn(ThreadStartFactory.Function(ReaderXY));
        var readerYX = Thread.Spawn(ThreadStartFactory.Function(ReaderYX));

        _gate.Release();

        LitmusAssert.ThreadSucceeded(writerX.Join(), "iriw writer x");
        LitmusAssert.ThreadSucceeded(writerY.Join(), "iriw writer y");
        LitmusAssert.ThreadSucceeded(readerXY.Join(), "iriw reader xy");
        LitmusAssert.ThreadSucceeded(readerYX.Join(), "iriw reader yx");

        var result = CoreIntrinsics.DefaultValue<DoublePair>();
        result.Left.First = _readerXYFirst;
        result.Left.Second = _readerXYSecond;
        result.Right.First = _readerYXFirst;
        result.Right.Second = _readerYXSecond;
        return result;
    }

    private static void WriterX()
    {
        _gate.Wait();
        if (!_writerXFirst)
        {
            LitmusSpin.Delay();
        }
        _x = 1;
    }

    private static void WriterY()
    {
        _gate.Wait();
        if (_writerXFirst)
        {
            LitmusSpin.Delay();
        }
        _y = 1;
    }

    private static void ReaderXY()
    {
        _gate.Wait();
        _readerXYFirst = _x;
        _readerXYSecond = _y;
    }

    private static void ReaderYX()
    {
        _gate.Wait();
        _readerYXFirst = _y;
        _readerYXSecond = _x;
    }
}

testcase IriwRejectsInconsistentReads()
{
    var writeXFirst = true;
    for (var iteration = 0; iteration < 512; iteration += 1)
    {
        var outcome = IriwScenario.RunIteration(writeXFirst);
        writeXFirst = !writeXFirst;
        var inconsistent =
            outcome.Left.First == 1
            && outcome.Left.Second == 0
            && outcome.Right.First == 0
            && outcome.Right.Second == 1;
        inconsistent = inconsistent
            || (
                outcome.Left.First == 0
                && outcome.Left.Second == 1
                && outcome.Right.First == 1
                && outcome.Right.Second == 0
            );
        LitmusAssert.Forbid(
            inconsistent,
            "IRIW observed inconsistent ordering across readers"
        );
    }
}
