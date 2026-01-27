namespace Unicode.Names;

public static class Program
{
    public static int Main()
    {
        if (Symbols.π <= 3.14) { return 1; }
        if (Symbols.你好 != "你好世界") { return 2; }
        if (Symbols.🐶🐮 != "dogcow") { return 3; }
        let area = Symbols.π * 2.0;
        return area > 6.28 ? 0 : 4;
    }
}
