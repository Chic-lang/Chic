namespace Std.Async;
/// Minimal unchecked numeric casts for std.async (kept local to avoid pulling in higher std layers).
internal static class NumericUnchecked
{
    public static int ToInt32(usize value) {
        unchecked {
            return(int) value;
        }
    }
    public static int ToInt32(isize value) {
        unchecked {
            return(int) value;
        }
    }
    public static usize ToUSize(usize value) {
        return value;
    }
    public static usize ToUSize(isize value) {
        unchecked {
            return(usize) value;
        }
    }
    public static usize ToUSize(int value) {
        unchecked {
            return(usize) value;
        }
    }
}
