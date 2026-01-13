namespace Spec.OptionalParameters;

public static class Recorder
{
    private static int sequence;

    public static void Reset()
    {
        sequence = 0;
    }

    public static int Capture(int value)
    {
        sequence = (sequence * 10) + value;
        return value;
    }

    public static int Sequence => sequence;
}

public class Widget
{
    public int Width;
    public int Height;

    public init(int width = 11, int height = 7)
    {
        Width = width;
        Height = height;
    }
}

public static class Runner
{
    public static int Combine(int start, int delta = 5, int scale = 2)
    {
        return (start + delta) * scale;
    }

    public static int UseNamed(int start, int scale = 4)
    {
        return Combine(start, scale: scale);
    }

    public static int EvaluateThunks(int start)
    {
        Recorder.Reset();
        var result = Combine(start, delta: Recorder.Capture(1), scale: Recorder.Capture(2));
        return result;
    }
}

public static class Program
{
    private static int ValidateConstructors()
    {
        var widget = new Widget();
        if (widget.Width != 11 || widget.Height != 7)
        {
            return 1;
        }
        var named = new Widget(height: 3);
        if (named.Width != 11 || named.Height != 3)
        {
            return 2;
        }
        return 0;
    }

    private static int ValidateFunctions()
    {
        if (Runner.Combine(2) != 14)
        {
            return 3;
        }
        if (Runner.UseNamed(3) != 32)
        {
            return 4;
        }
        var thunked = Runner.EvaluateThunks(1);
        if (thunked != 4)
        {
            return 5;
        }
        if (Recorder.Sequence != 12)
        {
            return 6;
        }
        return 0;
    }

    public static int Main()
    {
        var ctor = ValidateConstructors();
        if (ctor != 0)
        {
            return ctor;
        }
        var funcs = ValidateFunctions();
        if (funcs != 0)
        {
            return funcs;
        }
        return 0;
    }
}
