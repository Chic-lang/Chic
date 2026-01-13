namespace FnPtr;

public int IntValue()
{
    return 10;
}

public int InvokeInt(fn() -> int callback)
{
    return callback();
}

public bool InvokeBool(fn() -> bool callback)
{
    return callback();
}

public int LambdaValue()
{
    return 3;
}

public int AlternateValue()
{
    return 9;
}

public bool ReturnFalse()
{
    return false;
}

public bool ReturnTrue()
{
    return true;
}

public int Main()
{
    fn() -> int direct = IntValue;
    var total = direct();

    fn() -> int lambda = LambdaValue;
    total += InvokeInt(lambda);

    fn() -> int alt = AlternateValue;
    bool pick_first = false;
    fn() -> int chosen;
    if (pick_first)
    {
        chosen = direct;
    }
    else
    {
        chosen = alt;
    }
    total += chosen();

    fn() -> bool false_fn = ReturnFalse;
    fn() -> bool true_fn = ReturnTrue;
    bool pick_true = false;
    fn() -> bool bool_ptr;
    if (pick_true)
    {
        bool_ptr = true_fn;
    }
    else
    {
        bool_ptr = false_fn;
    }
    bool_ptr = true_fn;

    if (!bool_ptr())
    {
        return 91;
    }

    if (!InvokeBool(bool_ptr))
    {
        return 92;
    }

    return total;
}
