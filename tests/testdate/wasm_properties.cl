namespace Exec;

public class Counter
{
    private int _backing;

    public int Value { get; set; }

    public int Backing
    {
        get => _backing;
        private set => _backing = value;
    }

    public int Total { get; init; }

    public init()
    {
        Total = 7;
    }

    public int Run(ref this)
    {
        Value = 21;
        Backing = Value;
        if (Value != 21) { return -1; }
        if (Backing != 21) { return -2; }
        if (Total != 7) { return -3; }
        return Value;
    }
}

public int Main()
{
    var counter = new Counter();
    var result = counter.Run();
    if (result != 21) { return 1; }
    if (counter.Total != 7) { return 2; }
    return 0;
}
