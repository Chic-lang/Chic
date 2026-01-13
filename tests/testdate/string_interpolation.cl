import Std;
import Std.Strings;

namespace Exec;

public static string Compose(string name, long big, int small)
{
    // Mix owned strings with 64-bit and 32-bit numbers to exercise interpolation helpers.
    return $"name={name};big={big};small={small}";
}

public int Main()
{
    string name = "Chic";
    long big = 0x1_0000_0000L; // forces 64-bit formatting path
    int small = 7;
    u128 wide = 0x1_0000_0000_0000_0000u128; // exercises 128-bit hi/lo split
    i128 neg = -42i128;

    string message = Compose(name, big, small);
    if (message != "name=Chic;big=4294967296;small=7")
    {
        return 1;
    }

    string wrapped = $"wrap[{message}]";
    if (wrapped != "wrap[name=Chic;big=4294967296;small=7]")
    {
        return 2;
    }

    string wideMessage = $"wide={wide};neg={neg}";
    if (wideMessage != "wide=18446744073709551616;neg=-42")
    {
        return 3;
    }

    string nested128 = $"wrap128[{wideMessage}]";
    if (nested128 != "wrap128[wide=18446744073709551616;neg=-42]")
    {
        return 4;
    }

    return 0;
}
