namespace Exec;

public int Factorial(int value)
{
    if (value <= 1) { return 1; }
    return value * Factorial(value - 1);
}

public int Main()
{
    return Factorial(5) - 120;
}
