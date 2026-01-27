namespace Samples.Uri;

import Std;

public static class Program
{
    public static int Main()
    {
        TestAbsoluteRelative();
        TestComponents();
        TestEscaping();
        TestResolution();
        TestEquality();
        TestFileUnc();
        TestBuilder();
        TestValidation();
        return 0;
    }

    private static void TestAbsoluteRelative()
    {
        var abs = new Uri("http://example.com/path?x=1#f", UriKind.Absolute);
        Assert(abs.IsAbsoluteUri, "absolute flag");

        var rel = new Uri("a/b", UriKind.Relative);
        Assert(!rel.IsAbsoluteUri, "relative flag");

        var threw = false;
        try
        {
            var bad = new Uri("http://example.com", UriKind.Relative);
        }
        catch (UriFormatException)
        {
            threw = true;
        }
        Assert(threw, "relative rejects absolute");

        var ok = Uri.TryCreate("relative/path", UriKind.Absolute, out var _);
        Assert(!ok, "TryCreate rejects relative for absolute kind");
    }

    private static void TestComponents()
    {
        var uri = new Uri("http://user:pass@Example.com:8080/dir/seg?query=1#frag", UriKind.Absolute);
        Assert(uri.Scheme == "http", "scheme");
        Assert(uri.Host == "example.com", "host lowercased");
        Assert(uri.Port == 8080, "port");
        Assert(!uri.IsDefaultPort, "is default port");
        Assert(uri.Authority == "example.com:8080", "authority");
        Assert(uri.UserInfo == "user:pass", "userinfo");
        Assert(uri.AbsolutePath == "/dir/seg", "absolute path");
        Assert(uri.PathAndQuery == "/dir/seg?query=1", "path and query");
        Assert(uri.Query == "?query=1", "query");
        Assert(uri.Fragment == "#frag", "fragment");
        Assert(uri.AbsoluteUri == "http://example.com:8080/dir/seg?query=1#frag", "absolute uri");
        Assert(uri.HostNameType == UriHostNameType.Dns, "host name type");

        let segments = uri.Segments;
        Assert(segments.Length == 3, "segments length");
        Assert(segments[0] == "/", "segments root");
        Assert(segments[1] == "dir/", "segments dir");
        Assert(segments[2] == "seg", "segments leaf");

        let left = uri.GetLeftPart(UriPartial.Authority);
        Assert(left == "http://user:pass@example.com:8080", "left part authority");
        let comp = uri.GetComponents(UriComponents.PathAndQuery, UriFormat.UriEscaped);
        Assert(comp == "/dir/seg?query=1", "get components path+query");

        var idn = new Uri("http://bücher.example/path", UriKind.Absolute);
        Assert(idn.Host == "bücher.example", "idn host unicode");
        Assert(idn.IdnHost == "xn--bcher-kva.example", "idn host punycode");
        Assert(idn.DnsSafeHost == "xn--bcher-kva.example", "idn dns safe host");
        Assert(idn.AbsoluteUri == "http://xn--bcher-kva.example/path", "idn absolute uri");
    }

    private static void TestEscaping()
    {
        let escaped = Uri.EscapeDataString("a b?c");
        Assert(escaped == "a%20b%3Fc", "escape data string");
        let unescaped = Uri.UnescapeDataString("a%20b%3F");
        Assert(unescaped == "a b?", "unescape data string");
        let uriEscaped = Uri.EscapeUriString("http://example.com/a b");
        Assert(uriEscaped == "http://example.com/a%20b", "escape uri string");

        let hex = Uri.HexEscape(' ');
        Assert(hex == "%20", "hex escape");
        var index = 0;
        let ch = Uri.HexUnescape("%20", ref index);
        Assert(ch == ' ', "hex unescape");
        Assert(index == 3, "hex unescape index");
        Assert(Uri.IsHexDigit('f'), "hex digit");
        Assert(Uri.IsHexEncoding("x%2F", 1), "hex encoding");
    }

    private static void TestResolution()
    {
        var baseUri = new Uri("http://example.com/a/b/c", UriKind.Absolute);
        var resolved = new Uri(baseUri, "../d?x=1#f");
        Assert(resolved.AbsoluteUri == "http://example.com/a/d?x=1#f", "resolve dot segments");

        var baseQuery = new Uri("http://example.com/path?base=1", UriKind.Absolute);
        var overrideQuery = new Uri(baseQuery, "?q=2");
        Assert(overrideQuery.AbsoluteUri == "http://example.com/path?q=2", "query override");
    }

    private static void TestEquality()
    {
        var a = new Uri("http://example.com/", UriKind.Absolute);
        var b = new Uri("HTTP://EXAMPLE.com:80", UriKind.Absolute);
        Assert(a.Equals(b), "equality");
        Assert(Uri.Compare(a, b, UriComponents.AbsoluteUri, UriFormat.UriEscaped, StringComparison.OrdinalIgnoreCase) == 0, "compare");

        var baseUri = new Uri("http://example.com/a/b/", UriKind.Absolute);
        var child = new Uri("http://example.com/a/b/c/d", UriKind.Absolute);
        Assert(baseUri.IsBaseOf(child), "is base of");
        var rel = baseUri.MakeRelativeUri(child);
        Assert(rel.ToString() == "c/d", "make relative");
    }

    private static void TestFileUnc()
    {
        var file = new Uri("file:///tmp/data.txt", UriKind.Absolute);
        Assert(file.IsFile, "file is file");
        Assert(!file.IsUnc, "file not unc");
        Assert(file.LocalPath == "/tmp/data.txt", "file local path");

        var unc = new Uri("file://server/share/f.txt", UriKind.Absolute);
        Assert(unc.IsFile, "unc is file");
        Assert(unc.IsUnc, "unc flag");
        Assert(unc.LocalPath == "//server/share/f.txt", "unc local path");
    }

    private static void TestBuilder()
    {
        var builder = new UriBuilder("example.com/path?x=1#f");
        Assert(builder.Scheme == "http", "builder default scheme");
        Assert(builder.Host == "example.com", "builder host");
        Assert(builder.Query == "?x=1", "builder query");
        builder.Port = 8080;
        let built = builder.Uri;
        Assert(built.AbsoluteUri == "http://example.com:8080/path?x=1#f", "builder uri");

        var withUser = new UriBuilder(new Uri("http://user:pass@example.com/dir/", UriKind.Absolute));
        Assert(withUser.UserName == "user", "builder username");
        Assert(withUser.Password == "pass", "builder password");
    }

    private static void TestValidation()
    {
        var threw = false;
        try
        {
            var badUser = new Uri("http://user^@example.com/", UriKind.Absolute);
        }
        catch (UriFormatException)
        {
            threw = true;
        }
        Assert(threw, "userinfo rejects invalid characters");

        threw = false;
        try
        {
            var badHost = new Uri("http://exa mple.com/", UriKind.Absolute);
        }
        catch (UriFormatException)
        {
            threw = true;
        }
        Assert(threw, "host rejects spaces");
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException("URI test failed: " + message);
        }
    }
}
