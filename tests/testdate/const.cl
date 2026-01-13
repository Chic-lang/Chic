namespace ConstDemo;

public const int NamespaceValue = 4;

public class Holder
{
    public const int TypeValue = NamespaceValue * 2;
}

constexpr int Triple(int value)
{
    return value * 3;
}

public const int Result = Triple(Holder.TypeValue + 1);

public int Main()
{
    const int Local = Result - NamespaceValue;
    return Local;
}
