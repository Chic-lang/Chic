namespace Exec;

class Widget
{
    public int Value;
    public Widget? Next;
}

int Increment(ref int counter, int value)
{
    counter += 1;
    return value;
}

public int Main()
{
    int rhsCalls = 0;
    int indexCalls = 0;

    Widget? missing = null;
    Widget target = new Widget();
    Widget? nullableTarget = target;
    target.Value = 2;

    missing?.Value = Increment(ref rhsCalls, 10);
    nullableTarget?.Value = Increment(ref rhsCalls, 3);
    nullableTarget?.Value += Increment(ref rhsCalls, 4);

    nullableTarget = null;
    nullableTarget?.Value += Increment(ref rhsCalls, 100);

    int[]? numbers = null;
    numbers?[Increment(ref indexCalls, 0)] = Increment(ref rhsCalls, 50);
    numbers = new int[1];
    numbers?[Increment(ref indexCalls, 0)] = Increment(ref rhsCalls, 7);

    Widget root = new Widget();
    root.Next = new Widget();
    root.Next.Value = 1;
    Widget? nullableRoot = root;
    nullableRoot?.Next?.Value += Increment(ref rhsCalls, 2);
    nullableRoot = null;
    nullableRoot?.Next?.Value += Increment(ref rhsCalls, 99);

    if (rhsCalls != 4)
    {
        return 11;
    }
    if (indexCalls != 1)
    {
        return 12;
    }
    if (target.Value != 7)
    {
        return 13;
    }
    if (numbers == null || numbers[0] != 7)
    {
        return 14;
    }
    if (root.Next == null || root.Next.Value != 3)
    {
        return 15;
    }

    return 0;
}
