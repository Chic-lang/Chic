import Std;

namespace Exec;

public int Main()
{
    Option<int> some = Option<int>.Some(5);
    int extracted;
    if (!some.IsSome(out extracted) || extracted != 5)
    {
        return 10;
    }

    Option<int> none = Option<int>.None();
    if (!none.IsNone())
    {
        return 11;
    }

    if (none.UnwrapOr(9) != 9)
    {
        return 12;
    }

    Result<int, int> ok = Result<int, int>.FromOk(7);
    int okValue;
    if (!ok.IsOk(out okValue) || okValue != 7)
    {
        return 13;
    }

    Result<int, int> err = Result<int, int>.FromErr(3);
    int errValue;
    if (!err.IsErr(out errValue) || errValue != 3)
    {
        return 14;
    }

    return 0;
}
