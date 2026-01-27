namespace Std.Numeric;
/// Shared resolver for converting culture identifiers into numeric culture data.
internal static class NumericCultureInfo
{
    public static NumericCultureData Resolve(string culture) {
        if (culture == null || culture.Length == 0 || EqualsIgnoreAsciiCase (culture, "invariant"))
        {
            return new NumericCultureData(NumericUnchecked.ToByte('.'), NumericUnchecked.ToByte(','), 3);
        }
        if (EqualsIgnoreAsciiCase (culture, "en-US"))
        {
            return new NumericCultureData(NumericUnchecked.ToByte('.'), NumericUnchecked.ToByte(','), 3);
        }
        if (EqualsIgnoreAsciiCase (culture, "fr-FR"))
        {
            return new NumericCultureData(NumericUnchecked.ToByte(','), NumericUnchecked.ToByte(' '), 3);
        }
        throw new Std.ArgumentException("Unsupported culture");
    }
    internal static bool EqualsIgnoreAsciiCase(string left, string right) {
        if (left == null || right == null)
        {
            return false;
        }
        if (left.Length != right.Length)
        {
            return false;
        }
        var index = 0;
        while (index <left.Length)
        {
            let l = ToUpperAscii(left[index]);
            let r = ToUpperAscii(right[index]);
            if (l != r)
            {
                return false;
            }
            index += 1;
        }
        return true;
    }
    internal static char ToUpperAscii(char value) {
        if (value >= 'a' && value <= 'z')
        {
            return NumericUnchecked.ToChar(NumericUnchecked.ToInt32(value) - 32);
        }
        return value;
    }
}
