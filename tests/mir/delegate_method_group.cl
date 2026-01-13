namespace Samples;

public delegate int BinaryOp(int a, int b);

public int Add(int a, int b) => a + b;

public int Main()
{
    let op = (BinaryOp)Add;
    return op(2, 3);
}
