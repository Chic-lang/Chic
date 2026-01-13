namespace Samples;

public delegate int IntUnary(int x);

public int Main()
{
    let maybe = (IntUnary?)null;
    // invoking null delegate should produce a deterministic failure; this test asserts runtime trap.
    return maybe(1);
}
