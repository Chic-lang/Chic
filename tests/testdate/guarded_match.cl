namespace Exec
{
    public int Main()
    {
        var value = 5;
        switch (value)
        {
            case var x when x > 3:
                return x;
            default:
                return value;
        }
    }
}
