namespace Exec;

import Std.Numeric;

public static class Hex
{
    public static byte[] Parse(string hex)
    {
        if (hex == null)
        {
            throw new Std.ArgumentNullException("hex");
        }
        if ((hex.Length % 2) != 0)
        {
            throw new Std.ArgumentException("hex length must be even");
        }
        var length = hex.Length / 2;
        var result = new byte[length];
        var i = 0;
        while (i < length)
        {
            let high = ValueOf(hex[i * 2]);
            let low = ValueOf(hex[i * 2 + 1]);
            result[i] = NumericUnchecked.ToByte((high << 4) | low);
            i += 1;
        }
        return result;
    }

    private static int ValueOf(char c)
    {
        if (c >= '0' && c <= '9')
        {
            return c - '0';
        }
        if (c >= 'a' && c <= 'f')
        {
            return 10 + (c - 'a');
        }
        if (c >= 'A' && c <= 'F')
        {
            return 10 + (c - 'A');
        }
        throw new Std.ArgumentException("invalid hex digit");
    }
}
