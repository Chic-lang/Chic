namespace ByRefDemo;

public void Produce(out int value)
{
    value = 40;
}

public void Increment(ref int value)
{
    value += 2;
}

public int Main()
{
    int result;
    Produce(out result);
    Increment(ref result);
    return result;
}
