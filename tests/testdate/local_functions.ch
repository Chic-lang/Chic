namespace Samples.Local;

public static class Program
{
    public static int Main()
    {
        let baseValue = 30;

        function int AddBase(int delta)
        {
            return baseValue + delta;
        }

        function int Twice(int value)
        {
            return value * 2;
        }

        let intermediate = Twice(6);
        return AddBase(intermediate);
    }
}
