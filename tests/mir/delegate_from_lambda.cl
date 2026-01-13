namespace Samples;

public delegate int IntUnary(int x);

public int Apply(IntUnary op, int value) => op(value);

public int Main()
{
    let square = (IntUnary)((int x) => x * x);
    return Apply(square, 5);
}
