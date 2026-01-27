// Record examples: positional constructor, equality, and patterns.

namespace Samples;

public record struct Point(int X, int Y);

public record struct LabeledPoint(int X, int Y)
{
    public string Label;
}

public bool UsesEquality()
{
    var a = new Point(1, 2);
    var b = new Point { X = 1, Y = 2 };
    return a == b; // auto-generated Equatable/Hashable extension
}

public int MatchPoint(Point value)
{
    return value switch
    {
        Point(0, 0) => 0,
        Point(var x, var y) => x + y,
        _ => -1,
    };
}
